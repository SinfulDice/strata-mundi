#![allow(clippy::unwrap_used)]

use super::tests::{World, block, world};
use super::*;
use strata_core::inventory::{Click, HOTBAR, MAIN, SlotRef};
use strata_protocol::InventoryRequest;

fn total(w: &World) -> u32 {
    let inventory = w.game.inventory();
    let slots: u32 = inventory
        .slots
        .slots
        .iter()
        .flatten()
        .map(|s| u32::from(s.count))
        .sum();
    let carried = inventory.cursor.map_or(0, |s| u32::from(s.count));
    let chests: u32 = w
        .game
        .block_entities
        .values()
        .flat_map(|e| e.container.slots.iter().flatten())
        .map(|s| u32::from(s.count))
        .sum();
    let ground: u32 = w
        .game
        .ecs
        .drops
        .iter()
        .map(|(_, d)| u32::from(d.stack.count))
        .sum();
    slots + carried + chests + ground
}

fn request(w: &mut World, request: InventoryRequest) {
    w.game
        .inventory_request(&w.loader, request, &|item, _| item.0.to_string());
}

/// A survival player beside an open double chest, with stacks on both sides.
fn open_double_chest() -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    let chest = block(&w.content, "strata:chest");
    let halves = [BlockPos::new(7, 64, 6), BlockPos::new(8, 64, 6)];
    for pos in halves {
        assert!(w.loader.set_block(pos, chest));
    }
    assert!(w.game.open_block(&w.loader, halves[0]));
    let item = |name: &str| w.content.items.key(&name.parse().unwrap()).unwrap();
    let (stone, dirt) = (item("strata:stone"), item("strata:dirt"));
    let rules = w.game.rules.clone();
    w.game.with_inventory(|inventory, _| {
        inventory.slots.slots[0] = Some(ItemStack::new(stone, 50, &rules));
        inventory.slots.slots[MAIN.start] = Some(ItemStack::new(dirt, 33, &rules));
        inventory.slots.slots[MAIN.start + 4] = Some(ItemStack::new(stone, 7, &rules));
    });
    for (pos, count) in halves.into_iter().zip([20, 64]) {
        let entity = w.game.block_entities.get_mut(&pos).unwrap();
        entity.container.slots[5] = Some(ItemStack::new(stone, count, &rules));
    }
    w
}

#[test]
fn clicks_never_create_or_lose_items() {
    let mut w = open_double_chest();
    let before = total(&w);
    let clicks = [
        (SlotRef::player(0), Click::Primary),
        (SlotRef::open(5), Click::Primary),
        (SlotRef::open(32), Click::Secondary),
        (SlotRef::open(40), Click::Secondary),
        (SlotRef::player(MAIN.start), Click::Primary),
        (SlotRef::open(5), Click::Gather),
        (SlotRef::player(HOTBAR.start + 3), Click::Primary),
        (SlotRef::player(MAIN.start + 4), Click::Transfer),
        (SlotRef::open(32), Click::Transfer),
        (SlotRef::open(5), Click::HotbarSwap(2)),
        (SlotRef::player(MAIN.start), Click::Secondary),
        (SlotRef::player(MAIN.start + 9), Click::Secondary),
        (SlotRef::open(53), Click::Primary),
        (SlotRef::player(2), Click::Transfer),
    ];
    for (slot, click) in clicks {
        request(&mut w, InventoryRequest::Click { slot, click });
        assert_eq!(total(&w), before, "{click:?} on {slot:?}");
    }
    let dirt = w
        .content
        .items
        .key(&"strata:dirt".parse().unwrap())
        .unwrap();
    w.game.with_inventory(|inventory, rules| {
        inventory.cursor = Some(ItemStack::new(dirt, 9, rules));
    });
    let with_cursor = total(&w);
    request(&mut w, InventoryRequest::Close);
    assert_eq!(
        total(&w),
        with_cursor,
        "the carried stack goes back on closing"
    );
    assert!(w.game.inventory().cursor.is_none());
}
