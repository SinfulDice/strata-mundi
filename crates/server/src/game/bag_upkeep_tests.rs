#![allow(clippy::unwrap_used)]

use super::bag_tests::{contents, fill, slot, stack, worn};
use super::storage_tests::{click, item, put, request, survival};
use super::tests::{World, wait};
use super::*;
use glam::DVec3;
use strata_core::bag::BagContents;
use strata_core::inventory::{Area, BACK, Click, ItemStack, SlotRef};
use strata_protocol::InventoryRequest;

#[test]
fn the_crafting_upgrade_lends_a_3x3_grid_while_the_bag_is_open() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    put(&mut w, 0, "strata:crafting_upgrade", 1);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    let grid = |w: &World| w.game.ecs.crafts.get(w.game.player).unwrap().grid.side;
    assert_eq!(grid(&w), 3);
    put(&mut w, 1, "strata:oak_planks", 8);
    for cell in [0, 1, 2, 3, 5, 6, 7, 8] {
        click(&mut w, SlotRef::player(1), Click::Primary);
        click(&mut w, SlotRef::craft(cell), Click::Secondary);
        click(&mut w, SlotRef::player(1), Click::Primary);
    }
    let result = SlotRef {
        area: Area::CraftResult,
        index: 0,
    };
    click(&mut w, result, Click::Primary);
    let chest = item(&w, "strata:chest");
    assert_eq!(w.game.inventory().cursor.map(|s| s.item), Some(chest));
    request(&mut w, InventoryRequest::Close);
    assert_eq!(grid(&w), 2, "the inventory's grid is back");
}

#[test]
fn the_tank_takes_and_gives_buckets_of_one_liquid() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    put(&mut w, 0, "strata:tank_upgrade", 1);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    let carry = |w: &mut World, name: &str| {
        let bucket = stack(w, name, 1);
        w.game
            .with_inventory(|inventory, _| inventory.cursor = Some(bucket));
    };
    let cursor = |w: &World| w.game.inventory().cursor.map(|s| s.item);
    carry(&mut w, "strata:water_bucket");
    request(&mut w, InventoryRequest::Tank);
    assert_eq!(cursor(&w), Some(item(&w, "strata:bucket")));
    carry(&mut w, "strata:lava_bucket");
    request(&mut w, InventoryRequest::Tank);
    assert_eq!(
        cursor(&w),
        Some(item(&w, "strata:lava_bucket")),
        "one liquid"
    );
    carry(&mut w, "strata:bucket");
    request(&mut w, InventoryRequest::Tank);
    assert_eq!(cursor(&w), Some(item(&w, "strata:water_bucket")));
    let bag = slot(&w, BACK).unwrap();
    assert_eq!(contents(&w, &bag).tank, None, "empty again");
}

#[test]
fn a_worn_bag_filters_and_destroys_only_what_comes_in_by_itself() {
    let mut w = survival();
    worn(&mut w, "strata:iron_backpack");
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    for upgrade in ["strata:filter_upgrade", "strata:void_upgrade"] {
        put(&mut w, 0, upgrade, 1);
        click(&mut w, SlotRef::player(0), Click::Transfer);
    }
    let (gravel, dirt) = (item(&w, "strata:gravel"), item(&w, "strata:dirt"));
    let set = |w: &mut World, slot: u16, cell, item| {
        request(w, InventoryRequest::SetFilter { slot, cell, item });
    };
    set(&mut w, 45, Some(0), Some(gravel));
    set(&mut w, 45, None, None);
    set(&mut w, 46, Some(0), Some(dirt));
    put(&mut w, 0, "strata:dirt", 5);
    click(&mut w, SlotRef::player(0), Click::Primary);
    click(&mut w, SlotRef::open(0), Click::Primary);
    put(&mut w, 1, "strata:dirt", 7);
    click(&mut w, SlotRef::player(1), Click::Transfer);
    put(&mut w, 2, "strata:gravel", 3);
    click(&mut w, SlotRef::player(2), Click::Transfer);
    let bag = slot(&w, BACK).unwrap();
    let held = contents(&w, &bag);
    assert_eq!(
        held.storage.count(dirt),
        5,
        "put by hand: kept; shift click: destroyed"
    );
    assert_eq!(held.storage.count(gravel), 0, "refused by the filter");
    assert_eq!(slot(&w, 2).map(|s| s.count), Some(3));
}

#[test]
fn a_bag_holding_something_never_expires_on_the_ground() {
    let mut w = survival();
    let selected = w.game.selected();
    put(&mut w, selected, "strata:leather_backpack", 1);
    fill(&mut w, selected, "strata:stone", 4);
    request(&mut w, InventoryRequest::Close);
    let full = slot(&w, selected).unwrap();
    let empty = stack(&w, "strata:leather_backpack", 1);
    let far = DVec3::new(2.5, 64.0, 2.5);
    w.game.spawn_drop(full, far, DVec3::ZERO);
    w.game.spawn_drop(empty, far + DVec3::X, DVec3::ZERO);
    for (_, drop) in w.game.ecs.drops.iter_mut() {
        drop.age = u32::MAX / 2;
    }
    wait(&mut w, 1);
    let left: Vec<ItemStack> = w
        .game
        .ecs
        .drops
        .iter()
        .map(|(_, d)| d.stack.clone())
        .collect();
    assert_eq!(left.len(), 1, "the empty bag expired");
    assert_eq!(
        contents(&w, &left[0])
            .storage
            .count(item(&w, "strata:stone")),
        4
    );
}

#[test]
fn the_client_sees_a_summary_of_each_bag() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    fill(&mut w, BACK, "strata:stone", 9);
    let sent = w
        .game
        .take_messages()
        .into_iter()
        .rev()
        .find_map(|m| match m {
            ServerMessage::Inventory(inventory) => Some(inventory),
            _ => None,
        });
    let bag = sent.unwrap().slots.get(BACK).cloned().unwrap();
    let rules = w.game.rules.bags.get(bag.item).unwrap();
    assert!(
        BagContents::read(&bag.data, &rules.layout)
            .0
            .storage
            .is_empty()
    );
    assert_eq!(BagContents::summary(&bag.data, &rules.layout), (9, 0));
}

#[test]
fn a_feeding_bag_feeds_a_hungry_player_without_waste() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    put(&mut w, 0, "strata:feed_upgrade", 1);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    put(&mut w, 1, "strata:apple", 3);
    click(&mut w, SlotRef::player(1), Click::Transfer);
    request(&mut w, InventoryRequest::Close);
    let apples = |w: &World| {
        contents(w, &slot(w, BACK).unwrap())
            .storage
            .count(item(w, "strata:apple"))
    };
    let food = |w: &mut World, value: Option<f32>| {
        let needs = w.game.ecs.needs.get_mut(w.game.player).unwrap();
        if let Some(value) = value {
            needs.food = value;
        }
        needs.food
    };
    food(&mut w, Some(18.0));
    wait(&mut w, 40);
    assert_eq!(apples(&w), 3, "an apple would be half wasted");
    food(&mut w, Some(10.0));
    wait(&mut w, 60);
    assert_eq!(apples(&w), 1, "fed twice, never above full");
    assert_eq!(food(&mut w, None), 18.0);
}
