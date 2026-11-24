#![allow(clippy::unwrap_used)]

use super::tests::{content, new_world, settings};
use super::*;
use crate::loader::tests::loader;
use crate::rules::BlockRules;
use strata_core::bag::{BagContents, Tank};
use strata_core::inventory::{BACK, Container, PlayerInventory};
use strata_core::storage::set_filter_cell;
use strata_protocol::GameMode;

#[test]
fn bags_worn_put_down_and_in_a_chest_come_back_with_all_they_hold() {
    let (saves, folder, meta) = new_world("bags");
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let mut persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let rules = BlockRules::new(&content);
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(rules.clone(), 5_000, body, GameMode::Survival);
    let item = |name: &str| content.items.key(&name.parse().unwrap()).unwrap();
    let stack = |name: &str, count| ItemStack::new(item(name), count, &content.items);

    let mut filter = stack("strata:filter_upgrade", 1);
    set_filter_cell(&mut filter, 0, Some(item("strata:stone")), &content.items);
    let mut held = BagContents {
        storage: Container::new(27),
        upgrades: Container::new(1),
        tank: Some(Tank {
            bucket: "strata:water_bucket".parse().unwrap(),
            buckets: 3,
        }),
    };
    held.storage.slots[5] = Some(stack("strata:stone", 33));
    held.upgrades.slots[0] = Some(filter);
    let mut backpack = stack("strata:leather_backpack", 1);
    held.write(&mut backpack.data);
    let mut inventory = PlayerInventory::default();
    inventory.slots.slots[BACK] = Some(backpack);
    game.set_inventory(inventory.clone());

    let mut pouch = stack("strata:seed_pouch", 1);
    let mut seeds = BagContents {
        storage: Container::new(9),
        ..BagContents::default()
    };
    seeds.storage.slots[0] = Some(stack("strata:poppy", 7));
    seeds.write(&mut pouch.data);
    let mut chest = BlockEntity {
        container: Container::new(27),
        ..BlockEntity::default()
    };
    chest.container.slots[0] = Some(pouch);
    let chest_at = BlockPos::new(3, 64, 3);
    game.block_entities.insert(chest_at, chest.clone());

    let mut put_down = BlockEntity {
        container: Container::new(27),
        upgrades: Container::new(1),
        ..BlockEntity::default()
    };
    put_down.container.slots[2] = Some(stack("strata:dirt", 9));
    BagContents::set_put_down_as(
        &mut put_down.data,
        content.items.identifier(item("strata:leather_backpack")),
    );
    let bag_at = BlockPos::new(5, 64, 3);
    game.block_entities.insert(bag_at, put_down.clone());
    persistence.save(&game, &mut loader(), &dimension, None);

    let text = std::fs::read_to_string(folder.dir.join("player.ron")).unwrap();
    assert!(
        text.contains("strata:stone"),
        "carried stacks are saved by name"
    );
    let reopened = Persistence::new(
        settings(&folder, folder.read_meta().unwrap()),
        content.clone(),
    );
    let start = reopened.player_start(&dimension, || DVec3::ZERO);
    assert_eq!(
        start.inventory, inventory,
        "worn, with its upgrade, filter and tank"
    );
    let mut back = Game::new(rules, 5_000, body, GameMode::Survival);
    back.set_block_entities(reopened.block_entities(&dimension));
    assert_eq!(back.block_entities[&chest_at], chest, "in a chest");
    assert_eq!(back.block_entities[&bag_at], put_down, "put down");
    std::fs::remove_dir_all(saves).unwrap();
}
