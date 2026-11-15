#![allow(clippy::unwrap_used)]

use super::tests::{content, new_world, settings};
use super::*;
use crate::loader::tests::loader;
use crate::rules::BlockRules;
use strata_core::inventory::DurabilityMode;
use strata_core::upgrade::{installed, with_area, with_upgrade};
use strata_protocol::GameMode;

#[test]
fn the_durability_option_and_a_broken_tool_come_back() {
    let (saves, folder, mut meta) = new_world("broken-tool");
    assert_eq!(meta.durability, DurabilityMode::Break, "the default");
    meta.durability = DurabilityMode::KeepBroken;
    folder.write_meta(&meta).unwrap();
    let meta = folder.read_meta().unwrap();
    assert_eq!(meta.durability, DurabilityMode::KeepBroken);

    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let mut persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(BlockRules::new(&content), 5_000, body, GameMode::Survival);
    let pickaxe = content
        .items
        .key(&"strata:iron_pickaxe".parse().unwrap())
        .unwrap();
    let mut broken = ItemStack::new(pickaxe, 1, &content.items);
    broken.wear(1_000);
    let mut inventory = PlayerInventory::default();
    inventory.slots.slots[0] = Some(broken.clone());
    game.set_inventory(inventory);
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(settings(&folder, folder.read_meta().unwrap()), content);
    assert_eq!(reopened.meta.durability, DurabilityMode::KeepBroken);
    let start = reopened.player_start(&dimension, || DVec3::ZERO);
    let back = start.inventory.slots.get(0).unwrap();
    assert!(back.is_broken(), "{back:?}");
    assert_eq!(back, &broken);
    std::fs::remove_dir_all(saves).unwrap();
}

#[test]
fn upgrades_and_the_hammer_area_are_saved_with_the_item() {
    let (saves, folder, meta) = new_world("upgraded-tool");
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let mut persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(BlockRules::new(&content), 5_000, body, GameMode::Survival);
    let hammer = content
        .items
        .key(&"strata:diamond_hammer".parse().unwrap())
        .unwrap();
    let hammer = ItemStack::new(hammer, 1, &content.items);
    let area = "strata:hammer_area".parse().unwrap();
    let sharp = "strata:sharpening".parse().unwrap();
    let upgraded = with_upgrade(&with_upgrade(&hammer, &area, 4), &sharp, 2);
    let upgraded = with_area(&upgraded, 2);
    let mut inventory = PlayerInventory::default();
    inventory.slots.slots[0] = Some(upgraded.clone());
    game.set_inventory(inventory);
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(settings(&folder, folder.read_meta().unwrap()), content);
    let start = reopened.player_start(&dimension, || DVec3::ZERO);
    let back = start.inventory.slots.get(0).unwrap();
    assert_eq!(back, &upgraded);
    assert_eq!(installed(back).len(), 2);
    std::fs::remove_dir_all(saves).unwrap();
}
