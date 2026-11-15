#![allow(clippy::unwrap_used)]

use super::tests::{World, block, mine, world};
use super::*;
use strata_core::content::{Face, ItemId};
use strata_core::inventory::ItemStack;
use strata_core::upgrade::with_upgrade;
use strata_protocol::InventoryRequest;

/// The floor under the player's feet.
const FLOOR: BlockPos = BlockPos::new(8, 63, 8);

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

/// A survival world, an iron hammer widened to `level` in hand.
fn with_hammer(level: u8) -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    let hammer = ItemStack::new(item(&w, "strata:iron_hammer"), 1, &w.game.rules);
    let area = "strata:hammer_area".parse().unwrap();
    let hammer = with_upgrade(&hammer, &area, level);
    w.game
        .with_inventory(|inventory, _| *inventory.selected_slot_mut() = Some(hammer));
    w.game.take_messages();
    w
}

fn air_around(w: &World) -> usize {
    (7..=9)
        .flat_map(|x| (7..=9).map(move |z| BlockPos::new(x, 63, z)))
        .filter(|&pos| w.loader.block(pos) == Some(BlockId::AIR))
        .count()
}

fn uses_left(w: &World) -> u16 {
    let inventory = w.game.ecs.inventories.get(w.game.player).unwrap();
    inventory.selected_stack().unwrap().durability.unwrap()
}

/// On the ground or already picked up.
fn cobblestone_gained(w: &World) -> u32 {
    let cobble = item(w, "strata:cobblestone");
    let drops = w.game.ecs.drops.iter();
    let lying: u32 = drops
        .filter(|(_, d)| d.stack.item == cobble)
        .map(|(_, d)| u32::from(d.stack.count))
        .sum();
    lying + w.game.inventory().slots.count(cobble)
}

#[test]
fn a_plain_hammer_breaks_one_block() {
    let mut w = with_hammer(1);
    w.game.choose_area(0);
    mine(&mut w, FLOOR);
    assert_eq!(air_around(&w), 1);
}

#[test]
fn a_widened_hammer_breaks_its_area_with_one_use_per_block() {
    let mut w = with_hammer(3);
    mine(&mut w, FLOOR);
    assert_eq!(air_around(&w), 9, "3×3 on the floor");
    assert_eq!(uses_left(&w), 500 - 9);
    assert_eq!(cobblestone_gained(&w), 9);
    assert!(w.game.ecs.drops.iter().count() <= 1, "one heap per item");
}

#[test]
fn a_smaller_area_unlocked_can_be_chosen() {
    let mut w = with_hammer(3);
    let request = InventoryRequest::ChooseArea(1);
    w.game
        .inventory_request(&w.loader, request, &|_, _| String::new());
    mine(&mut w, FLOOR);
    assert_eq!(air_around(&w), 3, "3×1");
    let refused = InventoryRequest::ChooseArea(9);
    w.game
        .inventory_request(&w.loader, refused, &|_, _| String::new());
    assert_eq!(
        w.game.held_area().width,
        3,
        "an area not unlocked is refused"
    );
}

#[test]
fn blocks_the_hammer_cannot_mine_stay() {
    let mut w = with_hammer(3);
    let dirt = BlockPos::new(7, 63, 7);
    let obsidian = BlockPos::new(9, 63, 9);
    assert!(w.loader.set_block(dirt, block(&w.content, "strata:dirt")));
    assert!(
        w.loader
            .set_block(obsidian, block(&w.content, "strata:obsidian"))
    );
    mine(&mut w, FLOOR);
    assert_eq!(air_around(&w), 7);
    assert_eq!(uses_left(&w), 500 - 7);
}

#[test]
fn the_hardest_block_of_the_area_sets_the_time() {
    let mut w = with_hammer(3);
    let ore = block(&w.content, "strata:iron_ore");
    assert!(w.loader.set_block(BlockPos::new(9, 63, 7), ore));
    w.game.start_break(&w.loader, FLOOR, Face::Up);
    let needed = w
        .game
        .ecs
        .controls
        .get(w.game.player)
        .unwrap()
        .mining
        .unwrap()
        .needed;
    let tool = w.game.held_tool();
    let mining = &w.game.rules.mining;
    assert_eq!(needed, mining.break_ticks(ore, tool.as_ref()).unwrap());
    let stone = block(&w.content, "strata:stone");
    assert!(needed > mining.break_ticks(stone, tool.as_ref()).unwrap());
}

#[test]
fn a_face_the_player_cannot_see_starts_nothing() {
    let mut w = with_hammer(3);
    w.game.start_break(&w.loader, FLOOR, Face::Down);
    assert!(
        w.game
            .ecs
            .controls
            .get(w.game.player)
            .unwrap()
            .mining
            .is_none()
    );
    w.game.start_break(&w.loader, FLOOR, Face::North);
    assert!(
        w.game
            .ecs
            .controls
            .get(w.game.player)
            .unwrap()
            .mining
            .is_none(),
        "stone in front of that face"
    );
}

#[test]
fn the_area_ends_when_the_hammer_breaks() {
    let mut w = with_hammer(3);
    w.game.with_inventory(|inventory, _| {
        let hammer = inventory.selected_slot_mut().as_mut().unwrap();
        hammer.durability = Some(3);
    });
    mine(&mut w, FLOOR);
    assert_eq!(air_around(&w), 3, "the hit block and two more");
    let hammer = item(&w, "strata:iron_hammer");
    assert_eq!(
        w.game.inventory().slots.count(hammer),
        0,
        "the hammer broke"
    );
}

#[test]
fn a_smaller_area_chosen_while_hitting_is_kept_at_the_end() {
    let mut w = with_hammer(3);
    w.game.start_break(&w.loader, FLOOR, Face::Up);
    let request = InventoryRequest::ChooseArea(1);
    w.game
        .inventory_request(&w.loader, request, &|_, _| String::new());
    for _ in 0..200 {
        w.game.tick(&mut w.loader);
    }
    assert_eq!(air_around(&w), 3, "3×1, not the 3×3 of the start");
}

#[test]
fn a_harder_block_put_since_the_start_stays() {
    let mut w = with_hammer(3);
    w.game.start_break(&w.loader, FLOOR, Face::Up);
    let ore = block(&w.content, "strata:iron_ore");
    assert!(w.loader.set_block(BlockPos::new(9, 63, 9), ore));
    let needed = w
        .game
        .ecs
        .controls
        .get(w.game.player)
        .unwrap()
        .mining
        .unwrap()
        .needed;
    for _ in 0..=needed {
        w.game.tick(&mut w.loader);
    }
    assert_eq!(air_around(&w), 8);
    assert_eq!(w.loader.block(BlockPos::new(9, 63, 9)), Some(ore));
}
