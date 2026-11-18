#![allow(clippy::unwrap_used)]

use super::tests::{World, block, wait, world};
use super::*;
use strata_core::content::ItemId;
use strata_core::inventory::{ACCESSORIES, ARMOR, Area, Click, ItemStack, SlotRef};
use strata_core::physics::MoveInput;
use strata_core::worn::WornStat;
use strata_protocol::InventoryRequest;

const SPOT: BlockPos = BlockPos::new(10, 64, 8);
const RESULT: SlotRef = SlotRef {
    area: Area::CraftResult,
    index: 0,
};

fn survival() -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    w.game.take_messages();
    w
}

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn stack(w: &World, name: &str, count: u16) -> ItemStack {
    ItemStack::new(item(w, name), count, &w.game.rules)
}

fn put(w: &mut World, index: usize, stack: ItemStack) {
    w.game
        .with_inventory(|inventory, _| inventory.slots.slots[index] = Some(stack));
}

fn lay(w: &mut World, index: usize, name: &str, count: u16) {
    let stack = stack(w, name, count);
    put(w, index, stack);
}

fn slot(w: &World, index: usize) -> Option<ItemId> {
    let inventory = w.game.ecs.inventories.get(w.game.player)?;
    inventory.slots.get(index).map(|s| s.item)
}

fn request(w: &mut World, request: InventoryRequest) {
    w.game
        .inventory_request(&w.loader, request, &|item, _| item.0.to_string());
}

fn shift_click(w: &mut World, index: usize) {
    let slot = SlotRef::player(index);
    request(
        w,
        InventoryRequest::Click {
            slot,
            click: Click::Transfer,
        },
    );
}

#[test]
fn accessories_go_to_their_own_slots_and_like_rings_count_once() {
    let mut w = survival();
    lay(&mut w, 9, "strata:ring_of_vitality", 1);
    lay(&mut w, 10, "strata:ring_of_vitality", 1);
    lay(&mut w, 11, "strata:wool_cloak", 1);
    for index in 9..12 {
        shift_click(&mut w, index);
    }
    let ring = item(&w, "strata:ring_of_vitality");
    assert_eq!(slot(&w, ACCESSORIES.start), Some(ring));
    assert_eq!(slot(&w, ACCESSORIES.start + 1), Some(ring));
    assert_eq!(
        slot(&w, ACCESSORIES.end - 1),
        Some(item(&w, "strata:wool_cloak"))
    );
    let worn = w.game.equipment().stats.clone();
    assert_eq!(
        worn.get(WornStat::MaxHealth),
        4.0,
        "two like rings count once"
    );
    assert_eq!(worn.get(WornStat::Insulation), 4.0);
    lay(&mut w, 12, "strata:iron_ingot", 1);
    let cell = SlotRef::player(ACCESSORIES.start + 2);
    request(
        &mut w,
        InventoryRequest::Click {
            slot: SlotRef::player(12),
            click: Click::Primary,
        },
    );
    request(
        &mut w,
        InventoryRequest::Click {
            slot: cell,
            click: Click::Primary,
        },
    );
    assert_eq!(
        slot(&w, ACCESSORIES.start + 2),
        None,
        "an ingot is no amulet"
    );
}

fn wear(w: &mut World, names: &[&str]) {
    let stacks: Vec<_> = names.iter().map(|name| stack(w, name, 1)).collect();
    for (index, stack) in ARMOR.zip(stacks) {
        put(w, index, stack);
    }
}

#[test]
fn set_bonuses_grow_with_the_matching_pieces_worn() {
    let mut w = survival();
    let pieces = [
        "strata:diamond_scale_helmet",
        "strata:diamond_scale_chestplate",
        "strata:diamond_scale_leggings",
        "strata:diamond_scale_boots",
    ];
    let base: Vec<u16> = pieces
        .iter()
        .map(|name| {
            let item = w.content.items.get(item(&w, name)).unwrap();
            item.gear.armor.as_ref().unwrap().protection
        })
        .collect();
    for count in 1..=4 {
        wear(&mut w, &pieces[..count]);
        let worn = w.game.equipment().stats.clone();
        let bonus = [0, 0, 1, 2, 3][count];
        let sum: u16 = base[..count].iter().sum();
        assert_eq!(worn.protection, sum + bonus, "{count} pieces");
        let set = worn.sets.first().unwrap();
        assert_eq!((set.worn, set.total), (count as u8, 4));
    }
}

fn walk_north(w: &mut World, ticks: u32) {
    for sequence in 0..ticks {
        let input = MoveInput {
            wish: [0.0, 1.0],
            ..MoveInput::default()
        };
        w.game.queue_input(sequence, input);
        wait(w, 1);
    }
}

fn climber() -> World {
    let mut w = survival();
    let stone = block(&w.content, "strata:stone");
    for y in 64..72 {
        for x in 6..12 {
            assert!(w.loader.set_block(BlockPos::new(x, y, 6), stone));
        }
    }
    lay(&mut w, ARMOR.end - 1, "strata:crampon_boots", 1);
    w
}

#[test]
fn crampon_boots_climb_walls_on_the_server_unless_the_world_says_no() {
    let mut w = climber();
    walk_north(&mut w, 40);
    let height = w.game.player_body().unwrap().position.y;
    assert!(height > 66.0, "climbed to {height}");
    let mut w = climber();
    w.game.set_gear_mobility(false);
    walk_north(&mut w, 40);
    let height = w.game.player_body().unwrap().position.y;
    assert!(height < 64.6, "option off: stays at {height}");
    assert!(w.game.movement().unwrap().mobility.climb.is_none());
}

#[test]
fn leaping_boots_and_the_runner_s_belt_give_their_abilities() {
    let mut w = survival();
    lay(&mut w, ARMOR.end - 1, "strata:leaping_boots", 1);
    lay(&mut w, ACCESSORIES.start + 3, "strata:dash_belt", 1);
    wait(&mut w, 1);
    let mobility = w.game.movement().unwrap().mobility;
    assert_eq!(mobility.air_jumps, 1);
    assert!(mobility.dash.is_some() && mobility.climb.is_none());
}

fn at_bench(bench: &str) -> World {
    let mut w = survival();
    assert!(w.loader.set_block(SPOT, block(&w.content, bench)));
    assert!(w.game.open_block(&w.loader, SPOT));
    w
}

fn at_jeweller() -> World {
    at_bench("strata:jeweller")
}

fn recipe_index(w: &World, name: &str) -> u16 {
    let wanted = item(w, name);
    let benches = &w.game.rules.benches;
    benches
        .list()
        .iter()
        .position(|recipe| recipe.result == wanted)
        .and_then(|index| u16::try_from(index).ok())
        .unwrap()
}

#[test]
fn the_jeweller_lays_a_chosen_recipe_from_the_inventory_and_makes_it() {
    let mut w = at_jeweller();
    let view = w.game.crafting_view().unwrap();
    assert!(view.cuts.len() >= 4, "the jeweller lists its recipes");
    lay(&mut w, 9, "strata:gold_ingot", 3);
    lay(&mut w, 10, "strata:copper_ingot", 2);
    let ring = recipe_index(&w, "strata:ring_of_vitality");
    request(&mut w, InventoryRequest::ChooseCut(ring));
    let view = w.game.crafting_view().unwrap();
    assert_eq!(view.chosen, Some(ring));
    let made = view.result.map(|s| s.item);
    assert_eq!(made, Some(item(&w, "strata:ring_of_vitality")));
    request(
        &mut w,
        InventoryRequest::Click {
            slot: RESULT,
            click: Click::Transfer,
        },
    );
    let inventory = w.game.inventory();
    assert_eq!(
        inventory.slots.count(item(&w, "strata:ring_of_vitality")),
        1
    );
    assert_eq!(inventory.slots.count(item(&w, "strata:gold_ingot")), 2);
    assert_eq!(inventory.slots.count(item(&w, "strata:copper_ingot")), 0);
    assert!(
        w.game.crafting_view().unwrap().result.is_none(),
        "nothing left"
    );
}

#[test]
fn the_jeweller_refuses_what_it_lacks_and_recipes_of_other_benches() {
    let mut w = at_jeweller();
    lay(&mut w, 9, "strata:gold_ingot", 1);
    let ring = recipe_index(&w, "strata:ring_of_vitality");
    request(&mut w, InventoryRequest::ChooseCut(ring));
    let view = w.game.crafting_view().unwrap();
    assert!(view.result.is_none());
    assert!(
        view.grid.is_empty(),
        "nothing laid without every ingredient"
    );
    request(&mut w, InventoryRequest::ChooseCut(u16::MAX));
    assert_eq!(w.game.crafting_view().unwrap().chosen, Some(ring));
}

#[test]
fn the_forge_burns_a_coal_for_a_plate_and_refuses_the_jeweller_s_recipes() {
    let mut w = at_bench("strata:forge");
    lay(&mut w, 9, "strata:iron_ingot", 10);
    lay(&mut w, 10, "strata:charcoal", 2);
    lay(&mut w, 11, "strata:gold_ingot", 3);
    lay(&mut w, 12, "strata:copper_ingot", 2);
    let ring = recipe_index(&w, "strata:ring_of_vitality");
    request(&mut w, InventoryRequest::ChooseCut(ring));
    assert_eq!(w.game.crafting_view().unwrap().chosen, None, "not here");
    let plate = recipe_index(&w, "strata:iron_plate_chestplate");
    request(&mut w, InventoryRequest::ChooseCut(plate));
    let click = Click::Transfer;
    request(
        &mut w,
        InventoryRequest::Click {
            slot: RESULT,
            click,
        },
    );
    let inventory = w.game.inventory();
    let count = |name: &str| inventory.slots.count(item(&w, name));
    assert_eq!(count("strata:iron_plate_chestplate"), 1);
    assert_eq!(count("strata:iron_ingot"), 2);
    assert_eq!(count("strata:charcoal"), 1);
    assert_eq!(count("strata:gold_ingot"), 3);
}
