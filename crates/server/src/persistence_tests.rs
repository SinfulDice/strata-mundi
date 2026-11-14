#![allow(clippy::unwrap_used)]

use super::*;
use crate::loader::tests::loader;
use crate::rules::BlockRules;
use crate::save;
use std::path::Path;
use strata_core::inventory::{DataValue, OFFHAND};
use strata_core::pack::PackSet;
use strata_protocol::{GameMode, NewWorld};

fn content() -> Arc<Content> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    Arc::new(Content::load(&packs, &PackSet::default()).0)
}

#[test]
fn a_saved_player_comes_back_as_left() {
    let saves = std::env::temp_dir().join(format!("strata-persist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&saves);
    let new = NewWorld {
        name: "Test".into(),
        seed: String::new(),
        mode: GameMode::Creative,
        border: 5_000,
        fluids: true,
        day_cycle: true,
        fire_spread: true,
        fire_destroy: true,
        generation: Default::default(),
    };
    let (folder, meta) = save::create(&saves, &new, 9, 100).unwrap();
    let settings = |meta| WorldSettings {
        meta,
        folder: Some(folder.clone()),
        autosave_ticks: 0,
        backups: None,
    };
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let mut persistence = Persistence::new(settings(meta), Arc::clone(&content));
    let body = Body::at(DVec3::new(3.5, 140.0, -7.5));
    let mut game = Game::new(BlockRules::new(&content), 5_000, body, GameMode::Creative);
    let item = |name: &str| content.items.key(&name.parse().unwrap()).unwrap();
    let mut flint = ItemStack::new(item("strata:flint_and_steel"), 1, &content.items);
    flint.wear(3);
    flint
        .data
        .set("strata:note".parse().unwrap(), DataValue::Text("x".into()));
    let mut inventory = PlayerInventory::default();
    inventory.slots.slots[0] = Some(flint);
    let stones = ItemStack::new(item("strata:stone"), 40, &content.items);
    inventory.slots.slots[20] = Some(stones);
    let torches = ItemStack::new(item("strata:torch"), 3, &content.items);
    inventory.slots.slots[OFFHAND] = Some(torches);
    inventory.toggle_lock(20);
    inventory.selected = 4;
    game.set_inventory(inventory.clone());
    game.set_look([1.0, -0.5]);
    persistence.count_tick();
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(settings(folder.read_meta().unwrap()), content);
    let start = reopened.player_start(&dimension, || DVec3::ZERO);
    assert_eq!(start.body.position, body.position);
    assert_eq!(start.look, [1.0, -0.5]);
    assert_eq!(start.inventory, inventory);
    assert_eq!(start.boat, None);
    assert_eq!(reopened.meta.play_ticks, 1);
    assert_eq!(reopened.meta.day_time, 1, "the time of day is saved");
    assert!(reopened.meta.last_played > 100);
    std::fs::remove_dir_all(saves).unwrap();
}

#[test]
fn boats_and_the_seat_come_back_and_old_worlds_have_none() {
    let saves = std::env::temp_dir().join(format!("strata-boats-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&saves);
    let new = NewWorld {
        name: "Boats".into(),
        seed: String::new(),
        mode: GameMode::Creative,
        border: 5_000,
        fluids: true,
        day_cycle: true,
        fire_spread: true,
        fire_destroy: true,
        generation: Default::default(),
    };
    let (folder, meta) = save::create(&saves, &new, 9, 100).unwrap();
    let settings = |meta| WorldSettings {
        meta,
        folder: Some(folder.clone()),
        autosave_ticks: 0,
        backups: None,
    };
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let mut persistence = Persistence::new(settings(meta), Arc::clone(&content));
    assert!(persistence.boats(&dimension).is_empty(), "no file yet");
    let rules = BlockRules::new(&content);
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(rules.clone(), 5_000, body, GameMode::Creative);
    let kind = |name: &str| content.entities.key(&name.parse().unwrap()).unwrap();
    let (birch, acacia) = (kind("strata:birch_boat"), kind("strata:acacia_boat"));
    let first = BoatBody::at(DVec3::new(4.5, 63.7, -2.5), 1.25);
    let second = BoatBody::at(DVec3::new(-8.5, 63.7, 3.5), 0.0);
    game.spawn_boat(birch, first);
    game.restore_boats(vec![Some((acacia, second))], Some(0));
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(settings(folder.read_meta().unwrap()), content);
    let boats = reopened.boats(&dimension);
    assert_eq!(boats, vec![Some((birch, first)), Some((acacia, second))]);
    let start = reopened.player_start(&dimension, || DVec3::ZERO);
    assert_eq!(start.boat, Some(1));
    let mut back = Game::new(rules, 5_000, start.body, GameMode::Creative);
    back.restore_boats(boats, start.boat);
    let (saved, seat) = back.saved_boats();
    assert_eq!((saved.len(), seat), (2, Some(1)));
    std::fs::remove_dir_all(saves).unwrap();
}

fn new_world(name: &str) -> (std::path::PathBuf, save::WorldFolder, WorldMeta) {
    let saves = std::env::temp_dir().join(format!("strata-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&saves);
    let new = NewWorld {
        name: name.into(),
        seed: String::new(),
        mode: GameMode::Survival,
        border: 5_000,
        fluids: true,
        day_cycle: true,
        fire_spread: true,
        fire_destroy: true,
        generation: Default::default(),
    };
    let (folder, meta) = save::create(&saves, &new, 9, 100).unwrap();
    (saves, folder, meta)
}

fn settings(folder: &save::WorldFolder, meta: WorldMeta) -> WorldSettings {
    WorldSettings {
        meta,
        folder: Some(folder.clone()),
        autosave_ticks: 0,
        backups: None,
    }
}

#[test]
fn a_world_from_before_inventories_keeps_its_hotbar() {
    let (saves, folder, meta) = new_world("old-hotbar");
    let old = PlayerData {
        dimension: "strata:terra".parse().unwrap(),
        body: Body::at(DVec3::new(0.5, 70.0, 0.5)),
        look: [0.0; 2],
        inventory: None,
        hotbar: vec![
            Some("strata:stone".parse().unwrap()),
            None,
            Some("strata:gone".parse().unwrap()),
        ],
        selected: 2,
        boat: None,
    };
    folder.write_player(&old).unwrap();
    let path = saves.join(&folder.id).join("player.ron");
    let text = std::fs::read_to_string(&path).unwrap();
    let without: String = text
        .lines()
        .filter(|l| !l.contains("inventory"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, without).unwrap();

    let content = content();
    let persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let start = persistence.player_start(&dimension, || DVec3::ZERO);
    let stone = content.items.key(&"strata:stone".parse().unwrap()).unwrap();
    assert_eq!(
        start.inventory.slots.get(0).map(|s| (s.item, s.count)),
        Some((stone, 1))
    );
    assert!(
        start.inventory.slots.get(2).is_none(),
        "an item whose pack is gone"
    );
    assert_eq!(start.inventory.selected, 2);
    assert_eq!(
        start.inventory.slots.len(),
        strata_core::inventory::PLAYER_SLOTS
    );
    std::fs::remove_dir_all(saves).unwrap();
}

#[test]
fn chests_and_items_on_the_ground_are_saved() {
    let (saves, folder, meta) = new_world("chests");
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let mut persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let mut game = Game::new(
        BlockRules::new(&content),
        5_000,
        Body::at(DVec3::new(0.5, 70.0, 0.5)),
        GameMode::Survival,
    );
    let stone = content.items.key(&"strata:stone".parse().unwrap()).unwrap();
    let mut chest = BlockEntity {
        container: strata_core::inventory::Container::new(27),
        ..BlockEntity::default()
    };
    chest.container.slots[26] = Some(ItemStack::new(stone, 12, &content.items));
    let pos = BlockPos::new(3, 64, -9);
    game.block_entities.insert(pos, chest.clone());
    game.block_entities
        .insert(BlockPos::new(0, 1, 0), BlockEntity::default());
    game.spawn_drop(
        ItemStack::new(stone, 5, &content.items),
        DVec3::new(9.5, 70.0, 2.5),
        DVec3::ZERO,
    );
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(settings(&folder, folder.read_meta().unwrap()), content);
    let entities = reopened.block_entities(&dimension);
    assert_eq!(entities.len(), 1, "empty containers are not written");
    assert_eq!(entities.get(&pos), Some(&chest));
    let drops = reopened.drops(&dimension);
    assert_eq!(drops.len(), 1);
    assert_eq!((drops[0].0.item, drops[0].0.count), (stone, 5));
    assert_eq!(drops[0].1.position, DVec3::new(9.5, 70.0, 2.5));
    std::fs::remove_dir_all(saves).unwrap();
}
