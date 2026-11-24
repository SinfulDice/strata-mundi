#![allow(clippy::unwrap_used)]

use super::storage_tests::{click, item, put, request, survival};
use super::tests::{World, block, wait};
use super::*;
use glam::DVec3;
use strata_core::bag::BagContents;
use strata_core::content::Face;
use strata_core::inventory::{Area, BACK, Click, ItemStack, SlotRef};
use strata_protocol::InventoryRequest;

const GROUND: BlockPos = BlockPos::new(8, 63, 6);
const PUT_DOWN: BlockPos = BlockPos::new(8, 64, 6);

pub(super) fn stack(w: &World, name: &str, count: u16) -> ItemStack {
    ItemStack::new(item(w, name), count, &w.game.rules)
}

/// What the bag stack holds.
pub(super) fn contents(w: &World, bag: &ItemStack) -> BagContents {
    let rules = w.game.rules.bags.get(bag.item).unwrap();
    BagContents::read(&bag.data, &rules.layout).0
}

fn slot(w: &World, index: usize) -> Option<ItemStack> {
    w.game.inventory().slots.get(index).cloned()
}

/// Every `name` in the world: inventory, cursor, items on the ground and block entities,
/// and inside every bag among them.
pub(super) fn total(w: &World, name: &str) -> u32 {
    let wanted = item(w, name);
    let inventory = w.game.inventory();
    let mut stacks: Vec<ItemStack> = inventory.slots.slots.iter().flatten().cloned().collect();
    stacks.extend(inventory.cursor.clone());
    stacks.extend(w.game.ecs.drops.iter().map(|(_, d)| d.stack.clone()));
    for entity in w.game.block_entities.values() {
        stacks.extend(entity.container.slots.iter().flatten().cloned());
    }
    let mut count = 0;
    while let Some(stack) = stacks.pop() {
        if stack.item == wanted {
            count += u32::from(stack.count);
        }
        if w.game.rules.bags.is_bag(stack.item) {
            stacks.extend(contents(w, &stack).stacks().cloned());
        }
    }
    count
}

/// Opens the bag in `index` and sends `count` of `name` into it (from the last hotbar
/// slot).
pub(super) fn fill(w: &mut World, index: usize, name: &str, count: u16) {
    request(w, InventoryRequest::OpenBag(index as u16));
    put(w, 8, name, count);
    click(w, SlotRef::player(8), Click::Transfer);
}

fn worn(w: &mut World, name: &str) {
    let bag = stack(w, name, 1);
    w.game
        .with_inventory(|inventory, _| inventory.slots.slots[BACK] = Some(bag));
}

#[test]
fn a_worn_backpack_keeps_what_goes_in_inside_its_item() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    fill(&mut w, BACK, "strata:stone", 40);
    request(&mut w, InventoryRequest::Close);
    let bag = slot(&w, BACK).unwrap();
    assert_eq!(
        contents(&w, &bag).storage.count(item(&w, "strata:stone")),
        40
    );
    assert!(slot(&w, 8).is_none());
    w.game.take_messages();
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    let opened = w.game.take_messages().into_iter().find_map(|m| match m {
        ServerMessage::ContainerOpened(open) => Some(open),
        _ => None,
    });
    let opened = opened.unwrap();
    assert_eq!((opened.upgrades_at, opened.contents.len()), (27, 28));
    assert_eq!(opened.bag, Some(bag.item));
}

#[test]
fn no_bag_goes_in_a_bag() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    for name in ["strata:iron_backpack", "strata:seed_pouch"] {
        put(&mut w, 0, name, 1);
        click(&mut w, SlotRef::player(0), Click::Transfer);
        assert!(slot(&w, 0).is_some(), "{name}: shift click refused");
        click(&mut w, SlotRef::player(0), Click::Primary);
        click(&mut w, SlotRef::open(0), Click::Primary);
        assert!(
            w.game.inventory().cursor.is_some(),
            "{name}: by hand refused"
        );
        click(&mut w, SlotRef::player(0), Click::Primary);
    }
    let bag = slot(&w, BACK).unwrap();
    assert!(contents(&w, &bag).storage.is_empty());
    let mut nested = stack(&w, "strata:leather_backpack", 1);
    let inner = BagContents {
        storage: strata_core::inventory::Container {
            slots: vec![Some(stack(&w, "strata:seed_pouch", 1))],
        },
        ..BagContents::default()
    };
    inner.write(&mut nested.data);
    w.game.set_mode(GameMode::Creative);
    request(
        &mut w,
        InventoryRequest::CreativeSet {
            index: 5,
            stack: Some(nested),
        },
    );
    assert!(slot(&w, 5).is_none(), "a client cannot make one either");
}

#[test]
fn the_open_bag_stays_in_its_slot() {
    let mut w = survival();
    put(&mut w, 2, "strata:leather_backpack", 1);
    fill(&mut w, 2, "strata:dirt", 10);
    click(&mut w, SlotRef::player(2), Click::Primary);
    click(&mut w, SlotRef::player(2), Click::Drop { whole: true });
    click(&mut w, SlotRef::player(5), Click::HotbarSwap(2));
    request(
        &mut w,
        InventoryRequest::Sort {
            area: Area::Player,
            order: strata_core::inventory::SortOrder::Type,
            language: "fr".into(),
        },
    );
    let bag = slot(&w, 2).unwrap();
    assert_eq!(bag.item, item(&w, "strata:leather_backpack"));
    assert!(w.game.inventory().cursor.is_none());
    assert_eq!(w.game.ecs.drops.len(), 0);
    assert_eq!(total(&w, "strata:dirt"), 10);
}

#[test]
fn putting_down_and_breaking_a_bag_never_copies_what_it_holds() {
    for mode in [GameMode::Survival, GameMode::Creative] {
        let mut w = survival();
        w.game.set_mode(mode);
        let selected = w.game.selected();
        put(&mut w, selected, "strata:leather_backpack", 1);
        fill(&mut w, selected, "strata:stone", 30);
        request(&mut w, InventoryRequest::Close);
        let placed = block(&w.content, "strata:placed_leather_backpack");
        wait(&mut w, 3);
        let changes = w.game.place_block(&mut w.loader, GROUND, Face::Up, placed);
        assert_eq!(changes.first(), Some(&(PUT_DOWN, placed)), "{mode:?}");
        assert!(slot(&w, selected).is_none(), "{mode:?}: moved, not copied");
        assert_eq!(total(&w, "strata:stone"), 30);
        w.game
            .set_block(&mut w.loader, PUT_DOWN, BlockId::AIR, &mut Vec::new());
        let drops: Vec<ItemStack> = w
            .game
            .ecs
            .drops
            .iter()
            .map(|(_, d)| d.stack.clone())
            .collect();
        assert_eq!(drops.len(), 1, "{mode:?}: the bag, not its contents");
        assert_eq!(drops[0].item, item(&w, "strata:leather_backpack"));
        assert_eq!(total(&w, "strata:stone"), 30);
        for (_, drop) in w.game.ecs.drops.iter_mut() {
            drop.body.position = DVec3::new(8.5, 64.0, 8.5);
        }
        wait(&mut w, 2);
        let picked = (0..9).find_map(|i| slot(&w, i)).unwrap();
        assert_eq!(
            contents(&w, &picked)
                .storage
                .count(item(&w, "strata:stone")),
            30
        );
        assert_eq!(total(&w, "strata:stone"), 30);
    }
}

#[test]
fn a_bag_thrown_keeps_what_it_holds() {
    let mut w = survival();
    worn(&mut w, "strata:iron_backpack");
    fill(&mut w, BACK, "strata:gravel", 12);
    request(&mut w, InventoryRequest::Close);
    click(&mut w, SlotRef::player(BACK), Click::Drop { whole: true });
    assert_eq!(w.game.ecs.drops.len(), 1);
    assert_eq!(total(&w, "strata:gravel"), 12);
    let thrown: Vec<(crate::ecs::Entity, ItemStack)> = (w.game.ecs.drops.iter())
        .map(|(e, d)| (e, d.stack.clone()))
        .collect();
    for (entity, _) in &thrown {
        w.game.remove_drop(*entity);
    }
    let thrown = thrown.into_iter().map(|(_, stack)| stack);
    // Thrown again as death will throw it (milestone 10 tests death itself).
    w.game.throw_all(thrown);
    assert_eq!(total(&w, "strata:gravel"), 12);
}

#[test]
fn pouches_fill_first_on_pickup_with_what_their_tag_lists() {
    let mut w = survival();
    put(&mut w, 4, "strata:seed_pouch", 1);
    let here = DVec3::new(8.5, 64.2, 8.5);
    let poppy = stack(&w, "strata:poppy", 5);
    w.game.spawn_drop(poppy, here, DVec3::ZERO);
    let dirt = stack(&w, "strata:dirt", 3);
    w.game.spawn_drop(dirt, here, DVec3::ZERO);
    wait(&mut w, 2);
    let pouch = slot(&w, 4).unwrap();
    assert_eq!(
        contents(&w, &pouch).storage.count(item(&w, "strata:poppy")),
        5
    );
    assert_eq!(w.game.inventory().slots.count(item(&w, "strata:dirt")), 3);
    assert_eq!(w.game.inventory().slots.count(item(&w, "strata:poppy")), 0);
}

#[test]
fn a_pickup_upgrade_takes_what_its_filter_lets_in() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    put(&mut w, 0, "strata:pickup_upgrade", 1);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    request(&mut w, InventoryRequest::Close);
    let here = DVec3::new(8.5, 64.2, 8.5);
    let stone = stack(&w, "strata:stone", 7);
    w.game.spawn_drop(stone, here, DVec3::ZERO);
    wait(&mut w, 2);
    let bag = slot(&w, BACK).unwrap();
    assert_eq!(
        contents(&w, &bag).storage.count(item(&w, "strata:stone")),
        7
    );
    assert_eq!(contents(&w, &bag).upgrades.len(), 1);
}

#[test]
fn a_bag_magnet_draws_items_to_the_player_unless_the_world_says_no() {
    let mut w = survival();
    worn(&mut w, "strata:leather_backpack");
    request(&mut w, InventoryRequest::OpenBag(BACK as u16));
    put(&mut w, 0, "strata:magnet_upgrade", 1);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    request(&mut w, InventoryRequest::Close);
    let away = DVec3::new(12.0, 64.0, 8.5);
    let dirt = stack(&w, "strata:dirt", 4);
    w.game.spawn_drop(dirt, away, DVec3::ZERO);
    w.game.set_storage_magnets(false);
    wait(&mut w, 20);
    assert_eq!(w.game.ecs.drops.len(), 1, "magnets off");
    w.game.set_storage_magnets(true);
    wait(&mut w, 20);
    assert_eq!(w.game.ecs.drops.len(), 0);
    assert_eq!(w.game.inventory().slots.count(item(&w, "strata:dirt")), 4);
}

#[test]
fn a_kit_raises_a_backpack_put_down_keeping_everything() {
    let mut w = survival();
    let selected = w.game.selected();
    put(&mut w, selected, "strata:leather_backpack", 1);
    fill(&mut w, selected, "strata:stone", 20);
    request(&mut w, InventoryRequest::Close);
    let placed = block(&w.content, "strata:placed_leather_backpack");
    wait(&mut w, 3);
    w.game.place_block(&mut w.loader, GROUND, Face::Up, placed);
    put(&mut w, selected, "strata:iron_backpack_kit", 1);
    wait(&mut w, 3);
    w.game.use_block(&mut w.loader, PUT_DOWN);
    let iron = block(&w.content, "strata:placed_iron_backpack");
    assert_eq!(w.loader.block(PUT_DOWN), Some(iron));
    assert_eq!(w.game.block_entities[&PUT_DOWN].container.len(), 45);
    w.game
        .set_block(&mut w.loader, PUT_DOWN, BlockId::AIR, &mut Vec::new());
    let bag = w
        .game
        .ecs
        .drops
        .iter()
        .map(|(_, d)| d.stack.clone())
        .next()
        .unwrap();
    assert_eq!(bag.item, item(&w, "strata:iron_backpack"));
    let held = contents(&w, &bag);
    assert_eq!((held.storage.len(), held.upgrades.len()), (45, 2));
    assert_eq!(held.storage.count(item(&w, "strata:stone")), 20);
}

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
