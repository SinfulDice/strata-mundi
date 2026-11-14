#![allow(clippy::unwrap_used)]

use super::tests::{World, block, wait, world};
use super::*;
use strata_core::content::{ItemId, RecipeId};
use strata_core::inventory::{Area, Click, ItemStack, SlotRef};
use strata_protocol::InventoryRequest;

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn recipe(w: &World, name: &str) -> RecipeId {
    w.content.recipes.key(&name.parse().unwrap()).unwrap()
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

/// Moves the stack of player slot `from` into grid cell `cell`.
fn lay(w: &mut World, from: usize, cell: usize) {
    click(w, SlotRef::player(from), Click::Primary);
    click(w, SlotRef::craft(cell), Click::Primary);
}

fn result() -> SlotRef {
    SlotRef {
        area: Area::CraftResult,
        index: 0,
    }
}

fn grid(w: &World) -> CraftWindow {
    w.game.ecs.crafts.get(w.game.player).cloned().unwrap()
}

/// Items of `name` anywhere: inventory, carried, grid.
fn total(w: &World, name: &str) -> u32 {
    let item = item(w, name);
    let inventory = w.game.inventory();
    let carried = inventory.cursor.iter().filter(|s| s.item == item);
    inventory.slots.count(item)
        + grid(w).grid.slots.count(item)
        + carried.map(|s| u32::from(s.count)).sum::<u32>()
}

#[test]
fn the_inventory_grid_crafts_and_uses_the_ingredients() {
    let mut w = survival();
    put(&mut w, 9, "strata:oak_log", 3);
    lay(&mut w, 9, 3);
    let view = w.game.crafting_view().unwrap();
    assert_eq!(view.side, 2);
    assert_eq!(
        view.result.map(|s| (s.item, s.count)),
        Some((item(&w, "strata:oak_planks"), 4))
    );
    click(&mut w, result(), Click::Primary);
    let carried = w.game.inventory().cursor.unwrap();
    assert_eq!(
        (carried.item, carried.count),
        (item(&w, "strata:oak_planks"), 4)
    );
    assert_eq!(total(&w, "strata:oak_log"), 2);
    click(&mut w, result(), Click::Transfer);
    assert_eq!(total(&w, "strata:oak_log"), 0);
    assert_eq!(
        total(&w, "strata:oak_planks"),
        12,
        "every log became four planks"
    );
    assert!(
        w.game
            .take_messages()
            .iter()
            .any(|m| matches!(m, ServerMessage::Crafting(_)))
    );
}

#[test]
fn closing_gives_the_grid_back() {
    let mut w = survival();
    put(&mut w, 9, "strata:dirt", 5);
    lay(&mut w, 9, 0);
    assert_eq!(w.game.inventory().slots.count(item(&w, "strata:dirt")), 0);
    request(&mut w, InventoryRequest::Close);
    assert_eq!(w.game.inventory().slots.count(item(&w, "strata:dirt")), 5);
    assert!(grid(&w).grid.slots.is_empty());
}

#[test]
fn an_unknown_recipe_works_and_is_learnt_with_a_hint_before() {
    let mut w = survival();
    let sandstone = recipe(&w, "strata:sandstone");
    assert!(!w.game.knows(sandstone));
    put(&mut w, 9, "strata:sand", 4);
    click(&mut w, SlotRef::player(9), Click::Primary);
    click(&mut w, SlotRef::craft(0), Click::Secondary);
    assert!(
        w.game.crafting_view().unwrap().hint,
        "part of an unknown recipe"
    );
    for cell in 1..4 {
        click(&mut w, SlotRef::craft(cell), Click::Secondary);
    }
    assert!(!w.game.crafting_view().unwrap().hint);
    w.game.take_messages();
    click(&mut w, result(), Click::Transfer);
    assert_eq!(total(&w, "strata:sandstone"), 1);
    assert!(w.game.knows(sandstone));
    let learnt = w.game.take_messages();
    assert!(learnt.contains(&ServerMessage::RecipeLearned(sandstone)));
}

#[test]
fn every_recipe_is_known_in_creative_or_with_the_option() {
    let mut w = survival();
    let barrel = recipe(&w, "strata:barrel");
    assert!(
        w.game.knows(recipe(&w, "strata:crafting_table")),
        "a starting one"
    );
    assert!(!w.game.knows(barrel));
    w.game.set_recipe_discovery(false);
    assert!(w.game.knows(barrel));
    w.game.set_recipe_discovery(true);
    w.game.set_mode(GameMode::Creative);
    assert!(w.game.knows(barrel));
    let known = w
        .game
        .take_messages()
        .into_iter()
        .any(|m| matches!(m, ServerMessage::Recipes { all: true, .. }));
    assert!(known);
}

#[test]
fn the_recipe_book_fills_the_grid_with_known_recipes_only() {
    let mut w = survival();
    put(&mut w, 0, "strata:birch_planks", 3);
    put(&mut w, 20, "strata:oak_planks", 9);
    put(&mut w, 30, "strata:sand", 8);
    let table = recipe(&w, "strata:crafting_table");
    request(
        &mut w,
        InventoryRequest::FillRecipe {
            recipe: table,
            all: false,
        },
    );
    let laid = grid(&w).grid;
    assert_eq!(laid.slots.slots.iter().flatten().count(), 4);
    assert_eq!(
        laid.slots.count(item(&w, "strata:oak_planks")),
        4,
        "the wood there is most of"
    );
    click(&mut w, result(), Click::Primary);
    assert_eq!(total(&w, "strata:crafting_table"), 1);
    let sandstone = recipe(&w, "strata:sandstone");
    request(
        &mut w,
        InventoryRequest::FillRecipe {
            recipe: sandstone,
            all: true,
        },
    );
    assert!(grid(&w).grid.slots.is_empty(), "not known yet");
    assert_eq!(total(&w, "strata:sand"), 8);
}

#[test]
fn a_crafting_table_opens_a_three_by_three_grid() {
    let mut w = survival();
    let pos = BlockPos::new(9, 64, 9);
    assert!(
        w.loader
            .set_block(pos, block(&w.content, "strata:crafting_table"))
    );
    put(&mut w, 9, "strata:fir_planks", 16);
    assert!(w.game.open_block(&w.loader, pos));
    assert_eq!(grid(&w).grid.side, 3);
    let chest = recipe(&w, "strata:chest");
    request(
        &mut w,
        InventoryRequest::FillRecipe {
            recipe: chest,
            all: true,
        },
    );
    assert_eq!(grid(&w).grid.slots.count(item(&w, "strata:fir_planks")), 16);
    click(&mut w, result(), Click::Transfer);
    assert_eq!(total(&w, "strata:chest"), 2);
    put(&mut w, 10, "strata:stick", 2);
    lay(&mut w, 10, 4);
    assert!(w.loader.set_block(pos, BlockId::AIR));
    wait(&mut w, 1);
    w.game.settle_block_entities(&w.loader, &[]);
    assert_eq!(grid(&w).grid.side, 2, "the table went away");
    assert_eq!(total(&w, "strata:stick"), 2);
}
