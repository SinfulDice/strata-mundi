#![allow(clippy::unwrap_used)]

use super::mining_tests::add_pickaxe;
use super::tests::{World, block, hold, mine, place, wait, world_with};
use super::*;
use strata_core::content::{Face, ItemId};
use strata_core::cooking::{FURNACE_FUEL, FURNACE_INPUT, FURNACE_MODULES, FURNACE_OUTPUT};
use strata_core::inventory::{Click, ItemStack, SlotRef};
use strata_protocol::InventoryRequest;

const SPOT: BlockPos = BlockPos::new(10, 64, 10);

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn survival() -> World {
    let mut w = world_with(add_pickaxe);
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

fn transfer(w: &mut World, slot: usize) {
    let click = InventoryRequest::Click {
        slot: SlotRef::player(slot),
        click: Click::Transfer,
    };
    request(w, click);
}

/// A furnace of `name` at `SPOT` (its front toward the player), opened.
fn open_furnace(w: &mut World, name: &str) {
    let furnace = block(&w.content, name);
    let changes = place(w, SPOT.neighbour(Face::Down), Face::Up, furnace);
    assert_eq!(changes.len(), 1, "placed");
    wait(w, 2);
    assert!(w.game.open_block(&w.loader, SPOT));
}

fn slot_count(w: &World, index: usize) -> u16 {
    let entity = w.game.block_entities.get(&SPOT).unwrap();
    entity.container.get(index).map_or(0, |s| s.count)
}

fn furnace_block(w: &World) -> &strata_core::content::Block {
    w.content.blocks.get(w.loader.block(SPOT).unwrap()).unwrap()
}

#[test]
fn a_furnace_faces_the_player_cooks_with_coal_and_lights_up() {
    let mut w = survival();
    w.game.set_look([std::f32::consts::FRAC_PI_2, 0.0]);
    open_furnace(&mut w, "strata:furnace");
    let facing = furnace_block(&w).facing.unwrap();
    assert_eq!(
        facing.front,
        Face::East,
        "looking west, the front faces east"
    );
    assert!(!facing.lit);
    put(&mut w, 0, "strata:iron_ore", 3);
    put(&mut w, 1, "strata:coal", 2);
    put(&mut w, 2, "strata:dirt", 5);
    for slot in 0..3 {
        transfer(&mut w, slot);
    }
    assert_eq!(
        slot_count(&w, FURNACE_INPUT),
        3,
        "the ore goes to the input"
    );
    assert_eq!(slot_count(&w, FURNACE_FUEL), 2, "the coal to the fuel");
    assert_eq!(w.game.inventory().slots.count(item(&w, "strata:dirt")), 5);
    wait(&mut w, 2);
    let lit = furnace_block(&w).facing.unwrap();
    assert!(lit.lit && lit.front == Face::East, "lit, still facing east");
    assert!(furnace_block(&w).light_emission > 0);
    assert_eq!(slot_count(&w, FURNACE_FUEL), 1);
    let messages = w.game.take_messages();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, ServerMessage::Furnace(view) if view.burn_total == 1600)),
        "the open screen gets the flame"
    );
    wait(&mut w, 3 * 200);
    assert_eq!(slot_count(&w, FURNACE_OUTPUT), 3);
    assert_eq!(slot_count(&w, FURNACE_INPUT), 0);
    wait(&mut w, 1600);
    assert!(
        !furnace_block(&w).facing.unwrap().lit,
        "out of fire, out of light"
    );
    assert!(
        w.game.furnaces.is_empty(),
        "an idle furnace is not stepped any more"
    );
    let state = w.game.block_entities[&SPOT].furnace.clone().unwrap();
    assert!(
        (state.experience - 2.1).abs() < 1e-3,
        "experience kept for later"
    );
}

#[test]
fn a_kit_upgrades_a_furnace_in_place_keeping_everything() {
    let mut w = survival();
    open_furnace(&mut w, "strata:furnace");
    put(&mut w, 0, "strata:iron_ore", 10);
    put(&mut w, 1, "strata:coal", 4);
    transfer(&mut w, 0);
    transfer(&mut w, 1);
    wait(&mut w, 300);
    let kit = item(&w, "strata:gold_furnace_kit");
    hold(&mut w, Some(kit));
    let refused = w.game.use_block(&mut w.loader, SPOT);
    assert!(refused.is_empty(), "a gold kit needs an iron furnace");
    wait(&mut w, 2);
    let before = w.game.block_entities[&SPOT].clone();
    let state = before.furnace.clone().unwrap();
    assert!(state.progress > 0 && state.burn_left > 0, "cooking");
    let front = furnace_block(&w).facing.unwrap().front;
    let kit = item(&w, "strata:iron_furnace_kit");
    hold(&mut w, Some(kit));
    let changes = w.game.use_block(&mut w.loader, SPOT);
    assert_eq!(changes.len(), 1);
    let upgraded = w.loader.block(SPOT).unwrap();
    assert_eq!(
        w.game.rules.furnace(upgraded).unwrap().tier,
        "strata:iron".parse().unwrap()
    );
    let facing = furnace_block(&w).facing.unwrap();
    assert!(facing.lit && facing.front == front, "same front, still lit");
    let after = &w.game.block_entities[&SPOT];
    assert_eq!(after.furnace, before.furnace, "the cooking goes on");
    assert_eq!(
        after.container.len(),
        FURNACE_MODULES + 1,
        "one upgrade slot"
    );
    assert_eq!(
        after.container.slots[..FURNACE_MODULES],
        before.container.slots[..]
    );
    assert!(
        w.game.inventory().selected_stack().is_none(),
        "the kit is used"
    );
    assert!(
        w.game.ecs.windows.get(w.game.player).is_some(),
        "still open"
    );
    put(&mut w, 2, "strata:speed_module", 1);
    transfer(&mut w, 2);
    assert_eq!(
        slot_count(&w, FURNACE_MODULES),
        1,
        "the module goes in its slot"
    );
}

#[test]
fn a_better_furnace_cooks_faster_and_burns_less() {
    let cooked = |name: &str| {
        let mut w = survival();
        open_furnace(&mut w, name);
        put(&mut w, 0, "strata:iron_ore", 64);
        put(&mut w, 1, "strata:coal", 1);
        transfer(&mut w, 0);
        transfer(&mut w, 1);
        wait(&mut w, 700);
        let early = slot_count(&w, FURNACE_OUTPUT);
        wait(&mut w, 2000);
        (early, slot_count(&w, FURNACE_OUTPUT))
    };
    let (stone_early, stone) = cooked("strata:furnace");
    let (diamond_early, diamond) = cooked("strata:diamond_furnace");
    assert_eq!((stone_early, stone), (3, 8), "8 cookings per coal");
    assert_eq!(diamond, 16, "16 per coal");
    assert!(
        diamond_early > stone_early * 3,
        "more than three times faster"
    );
}

#[test]
fn a_broken_furnace_drops_everything_it_holds() {
    let mut w = survival();
    open_furnace(&mut w, "strata:iron_furnace");
    put(&mut w, 0, "strata:sand", 5);
    put(&mut w, 1, "strata:coal", 3);
    put(&mut w, 2, "strata:fuel_module", 1);
    for slot in 0..3 {
        transfer(&mut w, slot);
    }
    wait(&mut w, 250);
    let pickaxe = item(&w, "test:pickaxe");
    hold(&mut w, Some(pickaxe));
    assert!(!mine(&mut w, SPOT).is_empty());
    assert!(!w.game.block_entities.contains_key(&SPOT));
    let mut dropped: Vec<(ItemId, u16)> = w
        .game
        .ecs
        .drops
        .iter()
        .map(|(_, drop)| (drop.stack.item, drop.stack.count))
        .collect();
    dropped.sort();
    let mut expected = vec![
        (item(&w, "strata:sand"), 4),
        (item(&w, "strata:coal"), 2),
        (item(&w, "strata:fuel_module"), 1),
        (item(&w, "strata:glass"), 1),
        (item(&w, "strata:iron_furnace"), 1),
    ];
    expected.sort();
    assert_eq!(dropped, expected);
}

#[test]
fn the_stonecutter_cuts_the_variant_chosen() {
    let mut w = survival();
    assert!(
        w.loader
            .set_block(SPOT, block(&w.content, "strata:stonecutter"))
    );
    put(&mut w, 9, "strata:stone", 3);
    assert!(w.game.open_block(&w.loader, SPOT));
    request(
        &mut w,
        InventoryRequest::Click {
            slot: SlotRef::player(9),
            click: Click::Primary,
        },
    );
    request(
        &mut w,
        InventoryRequest::Click {
            slot: SlotRef::craft(0),
            click: Click::Primary,
        },
    );
    let view = w.game.crafting_view().unwrap();
    assert!(view.result.is_none(), "nothing chosen yet");
    let bricks = item(&w, "strata:stone_bricks");
    let (choice, _) = *view.cuts.iter().find(|(_, s)| s.item == bricks).unwrap();
    assert!(view.cuts.len() >= 2, "bricks and polished stone");
    request(&mut w, InventoryRequest::ChooseCut(choice));
    let view = w.game.crafting_view().unwrap();
    assert_eq!(view.result.map(|s| s.item), Some(bricks));
    let result = SlotRef {
        area: strata_core::inventory::Area::CraftResult,
        index: 0,
    };
    request(
        &mut w,
        InventoryRequest::Click {
            slot: result,
            click: Click::Transfer,
        },
    );
    assert_eq!(w.game.inventory().slots.count(bricks), 3);
    assert!(w.game.crafting_view().unwrap().grid.get(0).is_none());
    let planks = w.content.recipes.key(&"strata:oak_planks".parse().unwrap());
    let fill = InventoryRequest::FillRecipe {
        recipe: planks.unwrap(),
        all: false,
    };
    request(&mut w, fill);
    assert!(
        w.game.crafting_view().unwrap().cuts.is_empty(),
        "no grid recipe here"
    );
}
