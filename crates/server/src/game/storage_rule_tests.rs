#![allow(clippy::unwrap_used)]

use super::storage_tests::{
    A, B, chest, click, install, item, opened, put, request, storage_count, survival,
};
use super::tests::{block, hold, wait};
use super::*;
use strata_core::inventory::{Click, ItemStack, MAIN, SlotRef};
use strata_protocol::InventoryRequest;

fn set_cell(w: &mut tests::World, slot: u16, cell: u8, name: &str) {
    let item = item(w, name);
    let filter = InventoryRequest::SetFilter {
        slot,
        cell: Some(cell),
        item: Some(item),
    };
    request(w, filter);
}

#[test]
fn a_destruction_filter_never_takes_what_was_already_stored() {
    let mut w = survival();
    chest(&mut w, "strata:iron_chest", &[A]);
    let slot = install(&mut w, A, "strata:void_upgrade");
    put(&mut w, 0, "strata:dirt", 10);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    set_cell(&mut w, slot, 0, "strata:dirt");
    assert_eq!(storage_count(&w, &[A], "strata:dirt"), 10, "kept when set");
    let at = (0..45)
        .find(|&i| w.game.block_entities[&A].container.get(i).is_some())
        .unwrap();
    click(&mut w, SlotRef::open(at), Click::Primary);
    click(&mut w, SlotRef::open(30), Click::Primary);
    assert_eq!(
        storage_count(&w, &[A], "strata:dirt"),
        10,
        "moved inside: kept"
    );
    put(&mut w, 0, "strata:dirt", 5);
    click(&mut w, SlotRef::player(0), Click::Transfer);
    assert_eq!(
        storage_count(&w, &[A], "strata:dirt"),
        10,
        "only the five coming in go"
    );
    request(&mut w, InventoryRequest::Close);
    assert_eq!(
        storage_count(&w, &[A], "strata:dirt"),
        10,
        "closing destroys nothing"
    );
}

#[test]
fn an_open_furnace_screen_keeps_its_upgrade_slots_while_it_cooks() {
    let mut w = survival();
    let furnace = block(&w.content, "strata:iron_furnace");
    assert!(w.loader.set_block(A, furnace));
    assert!(w.game.open_block(&w.loader, A));
    let (upgrades_at, len) = opened(&mut w);
    assert_eq!((upgrades_at, len), (3, 4));
    put(&mut w, 0, "strata:speed_module", 1);
    put(&mut w, 1, "strata:iron_ore", 2);
    put(&mut w, 2, "strata:coal", 1);
    for slot in 0..3 {
        click(&mut w, SlotRef::player(slot), Click::Transfer);
    }
    w.game.take_messages();
    wait(&mut w, 200);
    let sizes: Vec<usize> = (w.game.take_messages().into_iter())
        .filter_map(|m| match m {
            ServerMessage::ContainerContents(contents) => Some(contents.len()),
            _ => None,
        })
        .collect();
    assert!(!sizes.is_empty(), "a cooking was sent");
    assert!(
        sizes.iter().all(|&n| n == 4),
        "input, fuel, output and the module: {sizes:?}"
    );
}

#[test]
fn a_filter_in_one_half_rules_the_quick_deposit_of_the_whole_double_chest() {
    let mut w = survival();
    chest(&mut w, "strata:iron_chest", &[A, B]);
    let slot = install(&mut w, A, "strata:filter_upgrade");
    set_cell(&mut w, slot, 0, "strata:stone");
    put(&mut w, 0, "strata:stone", 1);
    click(&mut w, SlotRef::player(0), Click::Primary);
    click(&mut w, SlotRef::open(2), Click::Primary);
    put(&mut w, 0, "strata:dirt", 1);
    click(&mut w, SlotRef::player(0), Click::Primary);
    click(&mut w, SlotRef::open(50), Click::Primary);
    request(&mut w, InventoryRequest::Close);
    put(&mut w, MAIN.start, "strata:stone", 20);
    put(&mut w, MAIN.start + 1, "strata:dirt", 20);
    request(&mut w, InventoryRequest::QuickDeposit);
    assert_eq!(storage_count(&w, &[A, B], "strata:stone"), 21);
    assert_eq!(
        storage_count(&w, &[A, B], "strata:dirt"),
        1,
        "the filter of the other half refuses it"
    );
}

#[test]
fn a_kit_that_would_part_a_double_chest_is_refused() {
    let mut w = survival();
    chest(&mut w, "strata:iron_chest", &[A, B]);
    let west = BlockPos::new(3, 64, 4);
    chest(&mut w, "strata:gold_chest", &[west]);
    let kit = item(&w, "strata:gold_storage_kit");
    w.game.with_inventory(|inventory, rules| {
        *inventory.selected_slot_mut() = Some(ItemStack::new(kit, 2, rules));
    });
    assert!(w.game.use_block(&mut w.loader, A).is_empty(), "refused");
    let iron = block(&w.content, "strata:iron_chest");
    assert_eq!(
        [w.loader.block(A), w.loader.block(B)],
        [Some(iron), Some(iron)]
    );
    assert_eq!(w.game.partner(&w.loader, A), Some(B));
    let kept = w.game.inventory().selected_stack().map(|s| s.count);
    assert_eq!(kept, Some(2), "no kit used");
    hold(&mut w, None);
}
