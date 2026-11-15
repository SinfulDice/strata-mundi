#![allow(clippy::unwrap_used)]

use super::tests::{content, new_world, settings};
use super::*;
use crate::loader::tests::loader;
use crate::rules::BlockRules;
use strata_core::inventory::{ACCESSORIES, DurabilityMode, OFFHAND, PLAYER_SLOTS};
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

#[test]
fn accessories_and_the_mobility_option_are_saved() {
    let (saves, folder, mut meta) = new_world("accessories");
    assert!(meta.gear_mobility, "on by default");
    meta.gear_mobility = false;
    folder.write_meta(&meta).unwrap();
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let meta = folder.read_meta().unwrap();
    let mut persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(BlockRules::new(&content), 5_000, body, GameMode::Survival);
    let item = |name: &str| content.items.key(&name.parse().unwrap()).unwrap();
    let mut inventory = PlayerInventory::default();
    let ring = ItemStack::new(item("strata:ring_of_vitality"), 1, &content.items);
    let cloak = ItemStack::new(item("strata:wool_cloak"), 1, &content.items);
    inventory.slots.slots[ACCESSORIES.start + 1] = Some(ring);
    inventory.slots.slots[ACCESSORIES.end - 1] = Some(cloak);
    game.set_inventory(inventory.clone());
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(settings(&folder, folder.read_meta().unwrap()), content);
    assert!(!reopened.meta.gear_mobility);
    let start = reopened.player_start(&dimension, || DVec3::ZERO);
    assert_eq!(start.inventory, inventory);
    std::fs::remove_dir_all(saves).unwrap();
}

#[test]
fn an_inventory_saved_before_accessories_gets_their_slots_empty() {
    let content = content();
    let item = |name: &str| content.items.key(&name.parse().unwrap()).unwrap();
    let mut old = PlayerInventory::default();
    old.slots.slots[OFFHAND] = Some(ItemStack::new(item("strata:torch"), 5, &content.items));
    let mut saved = SavedInventory::from_inventory(&old, &content.items);
    saved.slots.truncate(41);
    saved.locked.truncate(41);
    let text = ron::to_string(&saved).unwrap();
    let read: SavedInventory = ron::from_str(&text).unwrap();
    let mut back = read.to_inventory(&content.items);
    assert_eq!(back.slots.len(), PLAYER_SLOTS);
    assert_eq!(back, old, "same stacks, accessory slots empty");
    back.slots.slots[ACCESSORIES.start] = back.slots.slots[OFFHAND].take();
    let thrown = back.sanitize(&content.items);
    assert!(thrown.is_empty());
    assert!(
        back.slots.get(ACCESSORIES.start).is_none(),
        "a torch is no ring"
    );
    assert_eq!(back.slots.count(item("strata:torch")), 5);
}
