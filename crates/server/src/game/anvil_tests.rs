#![allow(clippy::unwrap_used)]

use super::tests::{World, block, world};
use super::*;
use strata_core::content::ItemId;
use strata_core::inventory::{Area, Click, ItemStack, SlotRef};
use strata_core::upgrade::installed;
use strata_protocol::InventoryRequest;

const SPOT: BlockPos = BlockPos::new(10, 64, 8);
const RESULT: SlotRef = SlotRef {
    area: Area::CraftResult,
    index: 0,
};

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn request(w: &mut World, request: InventoryRequest) {
    w.game
        .inventory_request(&w.loader, request, &|item, _| item.0.to_string());
}

fn click(w: &mut World, slot: SlotRef, click: Click) {
    request(w, InventoryRequest::Click { slot, click });
}

fn put(w: &mut World, index: usize, stack: ItemStack) {
    w.game
        .with_inventory(|inventory, _| inventory.slots.slots[index] = Some(stack));
}

fn stack(w: &World, name: &str, count: u16) -> ItemStack {
    ItemStack::new(item(w, name), count, &w.game.rules)
}

/// A survival world with an anvil opened at `SPOT`, `item` laid in its first cell and
/// `material` in the second.
fn at_anvil(item: ItemStack, material: Option<ItemStack>) -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    assert!(w.loader.set_block(SPOT, block(&w.content, "strata:anvil")));
    assert!(w.game.open_block(&w.loader, SPOT));
    put(&mut w, 9, item);
    click(&mut w, SlotRef::player(9), Click::Primary);
    click(&mut w, SlotRef::craft(0), Click::Primary);
    if let Some(material) = material {
        put(&mut w, 10, material);
        click(&mut w, SlotRef::player(10), Click::Primary);
        click(&mut w, SlotRef::craft(1), Click::Primary);
    }
    w
}

fn upgrade_index(w: &World, name: &str) -> u16 {
    w.game
        .rules
        .upgrades
        .index_of(&name.parse().unwrap())
        .unwrap()
}

#[test]
fn the_anvil_puts_the_upgrade_chosen() {
    let w = world();
    let sword = stack(&w, "strata:iron_sword", 1);
    let copper = stack(&w, "strata:copper_ingot", 3);
    let mut w = at_anvil(sword, Some(copper));
    let view = w.game.crafting_view().unwrap();
    assert!(view.result.is_none(), "a full sword: nothing to repair");
    let sharpening = upgrade_index(&w, "strata:sharpening");
    request(&mut w, InventoryRequest::ChooseUpgrade(Some(sharpening)));
    let view = w.game.crafting_view().unwrap();
    assert_eq!(view.chosen, Some(sharpening));
    assert_eq!(view.cost, Some(2));
    assert!(view.result.is_some());
    click(&mut w, RESULT, Click::Transfer);
    let inventory = w.game.inventory();
    let sword = inventory
        .slots
        .slots
        .iter()
        .flatten()
        .find(|s| s.item == item(&w, "strata:iron_sword"))
        .unwrap();
    assert_eq!(installed(sword).len(), 1);
    let view = w.game.crafting_view().unwrap();
    assert!(view.grid.get(0).is_none(), "the sword left the anvil");
    assert_eq!(view.grid.get(1).unwrap().count, 1, "two ingots paid");
}

#[test]
fn an_upgrade_the_item_cannot_take_is_not_chosen() {
    let w = world();
    let sword = stack(&w, "strata:iron_sword", 1);
    let mut w = at_anvil(sword, None);
    let lightening = upgrade_index(&w, "strata:lightening");
    request(&mut w, InventoryRequest::ChooseUpgrade(Some(lightening)));
    assert_eq!(w.game.crafting_view().unwrap().chosen, None);
}

#[test]
fn the_anvil_repairs_with_the_tier_material() {
    let w = world();
    let mut pick = stack(&w, "strata:iron_pickaxe", 1);
    pick.durability = Some(0);
    let ingots = stack(&w, "strata:iron_ingot", 2);
    let mut w = at_anvil(pick, Some(ingots));
    let view = w.game.crafting_view().unwrap();
    assert_eq!(
        view.result.unwrap().durability,
        Some(126),
        "two quarters of 250"
    );
    click(&mut w, RESULT, Click::Primary);
    let carried = w.game.inventory().cursor.unwrap();
    assert!(!carried.is_broken());
    assert!(w.game.crafting_view().unwrap().grid.get(1).is_none());
}

#[test]
fn closing_or_losing_the_anvil_gives_its_items_back() {
    let w = world();
    let sword = stack(&w, "strata:iron_sword", 1);
    let copper = stack(&w, "strata:copper_ingot", 3);
    let mut w = at_anvil(sword.clone(), Some(copper));
    request(&mut w, InventoryRequest::Close);
    let inventory = w.game.inventory();
    assert_eq!(inventory.slots.count(sword.item), 1);
    assert_eq!(inventory.slots.count(item(&w, "strata:copper_ingot")), 3);
    let mut w = at_anvil(sword.clone(), None);
    assert!(w.loader.set_block(SPOT, BlockId::AIR));
    w.game.tick(&mut w.loader);
    assert_eq!(w.game.inventory().slots.count(sword.item), 1);
}
