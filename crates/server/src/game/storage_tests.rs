#![allow(clippy::unwrap_used)]

use super::tests::{World, block, hold, wait, world};
use super::*;
use glam::DVec3;
use strata_core::content::ItemId;
use strata_core::inventory::{Click, ItemStack, SlotRef};
use strata_protocol::InventoryRequest;

const A: BlockPos = BlockPos::new(4, 64, 4);
const B: BlockPos = BlockPos::new(5, 64, 4);

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn survival() -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    w.game.take_messages();
    w
}

fn request(w: &mut World, request: InventoryRequest) {
    w.game
        .inventory_request(&w.loader, request, &|item, _| item.0.to_string());
}

fn put(w: &mut World, index: usize, name: &str, count: u16) {
    let stack = ItemStack::new(item(w, name), count, &w.game.rules);
    w.game
        .with_inventory(|inventory, _| inventory.slots.slots[index] = Some(stack));
}

fn click(w: &mut World, slot: SlotRef, click: Click) {
    request(w, InventoryRequest::Click { slot, click });
}

fn chest(w: &mut World, name: &str, at: &[BlockPos]) {
    let block = block(&w.content, name);
    for &pos in at {
        assert!(w.loader.set_block(pos, block));
    }
}

/// The open window's contents and where its upgrade slots start.
fn opened(w: &mut World) -> (u16, usize) {
    let messages = w.game.take_messages();
    let open = messages.into_iter().find_map(|m| match m {
        ServerMessage::ContainerOpened(open) => Some(open),
        _ => None,
    });
    let open = open.unwrap();
    (open.upgrades_at, open.contents.len())
}

fn storage_count(w: &World, at: &[BlockPos], name: &str) -> u32 {
    let item = item(w, name);
    at.iter()
        .map(|pos| w.game.block_entities[pos].container.count(item))
        .sum()
}

/// Opens the chest at `pos` and puts the upgrade `name` in its first upgrade slot.
fn install(w: &mut World, pos: BlockPos, name: &str) -> u16 {
    assert!(w.game.open_block(&w.loader, pos));
    let (upgrades_at, _) = opened(w);
    put(w, 0, name, 1);
    click(w, SlotRef::player(0), Click::Transfer);
    let installed = (upgrades_at..upgrades_at + 8).find(|&slot| {
        let entity = &w.game.block_entities[&pos];
        let index = usize::from(slot - upgrades_at);
        entity
            .upgrades
            .get(index)
            .is_some_and(|s| s.item == item(w, name))
    });
    installed.unwrap()
}

#[test]
fn a_kit_raises_a_double_chest_in_place_keeping_everything() {
    let mut w = survival();
    chest(&mut w, "strata:chest", &[A, B]);
    assert!(w.game.open_block(&w.loader, A));
    assert_eq!(
        opened(&mut w),
        (54, 56),
        "two halves of 27, one upgrade slot each"
    );
    put(&mut w, 0, "strata:stone", 40);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    put(&mut w, 1, "strata:magnet_upgrade", 1);
    click(&mut w, SlotRef::player(1), Click::Transfer);
    let kit = item(&w, "strata:iron_storage_kit");
    hold(&mut w, Some(kit));
    w.game.use_block(&mut w.loader, A);
    let iron = block(&w.content, "strata:iron_chest");
    assert_eq!(
        w.loader.block(A),
        Some(block(&w.content, "strata:chest")),
        "one kit, two blocks"
    );
    w.game.with_inventory(|inventory, rules| {
        *inventory.selected_slot_mut() = Some(ItemStack::new(kit, 2, rules));
    });
    wait(&mut w, 3);
    let changes = w.game.use_block(&mut w.loader, A);
    assert_eq!(changes.len(), 2, "both halves go up together");
    assert_eq!(
        [w.loader.block(A), w.loader.block(B)],
        [Some(iron), Some(iron)]
    );
    assert_eq!(
        w.game.partner(&w.loader, A),
        Some(B),
        "still one double chest"
    );
    assert!(
        w.game.inventory().selected_stack().is_none(),
        "two kits used"
    );
    assert_eq!(
        storage_count(&w, &[A, B], "strata:stone"),
        40,
        "contents kept"
    );
    let entity = &w.game.block_entities[&A];
    assert_eq!((entity.container.len(), entity.upgrades.len()), (54, 2));
    let magnet = item(&w, "strata:magnet_upgrade");
    assert!(
        entity
            .upgrades
            .slots
            .iter()
            .flatten()
            .any(|s| s.item == magnet)
    );
    let upgraded = w.game.take_messages().into_iter().find_map(|m| match m {
        ServerMessage::ContainerUpgraded { upgrades_at, .. } => Some(upgrades_at),
        _ => None,
    });
    assert_eq!(upgraded, Some(108), "the open screen follows");
}

#[test]
fn a_wrong_kit_changes_nothing() {
    let mut w = survival();
    chest(&mut w, "strata:barrel", &[A]);
    let kit = item(&w, "strata:gold_storage_kit");
    hold(&mut w, Some(kit));
    assert!(w.game.use_block(&mut w.loader, A).is_empty());
    assert_eq!(w.loader.block(A), Some(block(&w.content, "strata:barrel")));
    assert!(w.game.inventory().selected_stack().is_some(), "kit kept");
}

#[test]
fn upgrades_go_in_their_slots_once_and_only_on_storage() {
    let mut w = survival();
    chest(&mut w, "strata:gold_barrel", &[A]);
    assert!(w.game.open_block(&w.loader, A));
    let (upgrades_at, len) = opened(&mut w);
    assert_eq!((upgrades_at, len), (81, 84));
    for (slot, name) in [(0, "strata:sorting_upgrade"), (1, "strata:sorting_upgrade")] {
        put(&mut w, slot, name, 1);
        click(&mut w, SlotRef::player(slot), Click::Transfer);
    }
    let sorting = item(&w, "strata:sorting_upgrade");
    let entity = &w.game.block_entities[&A];
    let held = entity
        .upgrades
        .slots
        .iter()
        .flatten()
        .filter(|s| s.item == sorting);
    assert_eq!(held.count(), 1, "the same upgrade does not add up");
    assert_eq!(
        entity.container.count(sorting),
        1,
        "the second went to the storage"
    );
    put(&mut w, 2, "strata:speed_module", 1);
    click(&mut w, SlotRef::player(2), Click::Primary);
    click(
        &mut w,
        SlotRef::open(usize::from(upgrades_at) + 1),
        Click::Primary,
    );
    let speed = item(&w, "strata:speed_module");
    assert!(
        !w.game.block_entities[&A]
            .upgrades
            .slots
            .iter()
            .flatten()
            .any(|s| s.item == speed),
        "a furnace module is not a storage upgrade"
    );
}

#[test]
fn a_sorting_storage_sorts_itself_when_closed() {
    let mut w = survival();
    chest(&mut w, "strata:chest", &[A]);
    install(&mut w, A, "strata:sorting_upgrade");
    put(&mut w, 0, "strata:dirt", 5);
    click(&mut w, SlotRef::player(0), Click::Primary);
    click(&mut w, SlotRef::open(20), Click::Primary);
    put(&mut w, 0, "strata:dirt", 7);
    click(&mut w, SlotRef::player(0), Click::Primary);
    click(&mut w, SlotRef::open(9), Click::Primary);
    let dirt = item(&w, "strata:dirt");
    assert_eq!(
        w.game.block_entities[&A].container.get(20).map(|s| s.item),
        Some(dirt)
    );
    request(&mut w, InventoryRequest::Close);
    let container = &w.game.block_entities[&A].container;
    assert_eq!(
        container.get(0).map(|s| (s.item, s.count)),
        Some((dirt, 12))
    );
    assert!(container.get(20).is_none() && container.get(9).is_none());
}

#[test]
fn a_filter_lets_in_only_what_it_allows() {
    let mut w = survival();
    chest(&mut w, "strata:chest", &[A]);
    let slot = install(&mut w, A, "strata:filter_upgrade");
    let stone = item(&w, "strata:stone");
    request(
        &mut w,
        InventoryRequest::SetFilter {
            slot,
            cell: Some(0),
            item: Some(stone),
        },
    );
    put(&mut w, 0, "strata:stone", 10);
    put(&mut w, 1, "strata:dirt", 10);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    click(&mut w, SlotRef::player(1), Click::Transfer);
    assert_eq!(storage_count(&w, &[A], "strata:stone"), 10);
    assert_eq!(
        storage_count(&w, &[A], "strata:dirt"),
        0,
        "refused by the filter"
    );
    request(
        &mut w,
        InventoryRequest::SetFilter {
            slot,
            cell: None,
            item: None,
        },
    );
    put(&mut w, 0, "strata:stone", 10);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    click(&mut w, SlotRef::player(1), Click::Transfer);
    assert_eq!(storage_count(&w, &[A], "strata:stone"), 10, "now refused");
    assert_eq!(storage_count(&w, &[A], "strata:dirt"), 10, "now allowed");
}

#[test]
fn compression_presses_nuggets_into_ingots_and_ingots_into_blocks() {
    let mut w = survival();
    chest(&mut w, "strata:chest", &[A]);
    install(&mut w, A, "strata:compression_upgrade");
    put(&mut w, 0, "strata:iron_nugget", 64);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    put(&mut w, 0, "strata:iron_nugget", 26);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    assert_eq!(
        storage_count(&w, &[A], "strata:iron_block"),
        1,
        "81 nuggets"
    );
    assert_eq!(storage_count(&w, &[A], "strata:iron_ingot"), 1);
    assert_eq!(storage_count(&w, &[A], "strata:iron_nugget"), 0);
}

#[test]
fn destruction_takes_only_what_its_filter_names() {
    let mut w = survival();
    chest(&mut w, "strata:chest", &[A]);
    let slot = install(&mut w, A, "strata:void_upgrade");
    put(&mut w, 0, "strata:dirt", 10);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    assert_eq!(
        storage_count(&w, &[A], "strata:dirt"),
        10,
        "an empty filter destroys nothing"
    );
    let gravel = item(&w, "strata:gravel");
    request(
        &mut w,
        InventoryRequest::SetFilter {
            slot,
            cell: Some(4),
            item: Some(gravel),
        },
    );
    put(&mut w, 0, "strata:gravel", 30);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    assert_eq!(storage_count(&w, &[A], "strata:gravel"), 0, "destroyed");
    assert_eq!(storage_count(&w, &[A], "strata:dirt"), 10, "the rest stays");
}

#[test]
fn a_magnet_draws_the_items_around_unless_the_world_says_no() {
    let mut w = survival();
    chest(&mut w, "strata:chest", &[A]);
    install(&mut w, A, "strata:magnet_upgrade");
    request(&mut w, InventoryRequest::Close);
    let stack = |w: &World, name, n| ItemStack::new(item(w, name), n, &w.game.rules);
    let near = DVec3::new(6.5, 64.0, 6.5);
    let far = DVec3::new(14.5, 64.0, 14.5);
    w.game
        .spawn_drop(stack(&w, "strata:dirt", 5), near, DVec3::ZERO);
    w.game
        .spawn_drop(stack(&w, "strata:stone", 3), far, DVec3::ZERO);
    w.game.set_storage_magnets(false);
    wait(&mut w, 20);
    assert_eq!(storage_count(&w, &[A], "strata:dirt"), 0, "magnets off");
    w.game.set_storage_magnets(true);
    wait(&mut w, 20);
    assert_eq!(storage_count(&w, &[A], "strata:dirt"), 5);
    assert_eq!(storage_count(&w, &[A], "strata:stone"), 0, "too far");
    assert_eq!(w.game.ecs.drops.len(), 1);
}

#[test]
fn breaking_an_upgraded_chest_drops_its_upgrades_too() {
    let mut w = survival();
    chest(&mut w, "strata:chest", &[A]);
    install(&mut w, A, "strata:magnet_upgrade");
    put(&mut w, 0, "strata:dirt", 9);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    request(&mut w, InventoryRequest::Close);
    w.game
        .set_block(&mut w.loader, A, BlockId::AIR, &mut Vec::new());
    let changes = vec![(A, BlockId::AIR)];
    w.game.settle_block_entities(&w.loader, &changes);
    let dropped: Vec<ItemId> = w.game.ecs.drops.iter().map(|(_, d)| d.stack.item).collect();
    assert!(dropped.contains(&item(&w, "strata:magnet_upgrade")));
    assert!(dropped.contains(&item(&w, "strata:dirt")));
    assert!(w.game.magnets.at.is_empty(), "its magnet is gone");
}
