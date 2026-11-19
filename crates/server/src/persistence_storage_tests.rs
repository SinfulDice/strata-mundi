#![allow(clippy::unwrap_used)]

use super::tests::{content, new_world, settings};
use super::*;
use crate::loader::tests::loader;
use crate::rules::BlockRules;
use crate::save::SavedBlockEntity;
use strata_core::inventory::Container;
use strata_core::storage::{filter_of, set_filter_cell};
use strata_protocol::GameMode;

#[test]
fn furnace_modules_saved_after_the_fuel_move_to_the_upgrade_slots() {
    let (saves, folder, meta) = new_world("old-modules");
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let id = |name: &str| -> Identifier { name.parse().unwrap() };
    let stack = |name: &str, count| {
        Some(crate::save::SavedStack {
            item: id(name),
            count,
            durability: None,
            data: Default::default(),
        })
    };
    let old = SavedBlockEntity {
        pos: [1, 64, 1],
        slots: vec![
            stack("strata:iron_ore", 4),
            stack("strata:coal", 2),
            None,
            stack("strata:speed_module", 1),
        ],
        upgrades: Vec::new(),
        data: Default::default(),
        furnace: Some(Default::default()),
    };
    let data = crate::save::BlockEntitiesData {
        entities: vec![old],
    };
    folder.write_block_entities(&dimension, &data).unwrap();
    let persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let entities = persistence.block_entities(&dimension);
    let entity = &entities[&BlockPos::new(1, 64, 1)];
    let speed = content.items.key(&id("strata:speed_module")).unwrap();
    assert_eq!(entity.container.len(), 3, "input, fuel, output");
    assert_eq!(
        entity.upgrades.get(0).map(|s| s.item),
        Some(speed),
        "module kept"
    );

    let rules = BlockRules::new(&content);
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(rules.clone(), 5_000, body, GameMode::Survival);
    let furnace = content.blocks.key(&id("strata:iron_furnace")).unwrap();
    let layout = rules.layout(furnace).unwrap();
    assert_eq!((layout.storage, layout.upgrades), (3, 1));
    game.set_block_entities(entities);
    assert!(game.furnace_is_active(BlockPos::new(1, 64, 1)));
    std::fs::remove_dir_all(saves).unwrap();
}

#[test]
fn an_upgraded_chest_comes_back_with_its_upgrades_and_filter() {
    let (saves, folder, meta) = new_world("upgraded-chest");
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let mut persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let rules = BlockRules::new(&content);
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(rules.clone(), 5_000, body, GameMode::Survival);
    let item = |name: &str| content.items.key(&name.parse().unwrap()).unwrap();
    let mut chest = BlockEntity {
        container: Container::new(81),
        upgrades: Container::new(3),
        ..BlockEntity::default()
    };
    chest.container.slots[80] = Some(ItemStack::new(item("strata:diamond"), 12, &content.items));
    let mut filter = ItemStack::new(item("strata:filter_upgrade"), 1, &content.items);
    set_filter_cell(&mut filter, 2, Some(item("strata:diamond")), &content.items);
    chest.upgrades.slots[1] = Some(filter);
    chest.upgrades.slots[2] = Some(ItemStack::new(
        item("strata:magnet_upgrade"),
        1,
        &content.items,
    ));
    let pos = BlockPos::new(3, 64, 3);
    game.block_entities.insert(pos, chest.clone());
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(
        settings(&folder, folder.read_meta().unwrap()),
        content.clone(),
    );
    let mut back = Game::new(rules, 5_000, body, GameMode::Survival);
    back.set_block_entities(reopened.block_entities(&dimension));
    assert_eq!(
        back.block_entities[&pos], chest,
        "contents, upgrades and filter kept"
    );
    let filter = back.block_entities[&pos].upgrades.get(1).unwrap();
    let cells = filter_of(filter, &content.items).cells;
    assert_eq!(cells[2], Some(item("strata:diamond")));
    assert!(back.has_magnet(pos), "its magnet works again");
    std::fs::remove_dir_all(saves).unwrap();
}
