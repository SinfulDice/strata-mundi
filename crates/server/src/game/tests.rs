#![allow(clippy::unwrap_used)]

use super::*;
use crate::loader::tests::loader;
use std::path::Path;
use strata_core::chunk::Column;
use strata_core::content::{Content, ItemId};
use strata_core::coords::{ColumnPos, DEFAULT_WORLD_BORDER};
use strata_core::pack::PackSet;

pub(super) struct World {
    pub(super) game: Game,
    pub(super) loader: ColumnLoader,
    pub(super) content: Content,
}

/// Stone up to y = 63 in column (0, 0), the player standing on it at (8.5, 64, 8.5).
pub(super) fn world() -> World {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let (content, _) = Content::load(&packs, &PackSet::default());
    let stone = block(&content, "strata:stone");
    let mut column = Column::empty(ColumnPos { x: 0, z: 0 });
    for (x, y, z) in
        (0..16).flat_map(|x| (0..64).flat_map(move |y| (0..16).map(move |z| (x, y, z))))
    {
        column.set(x, y, z, stone);
    }
    let mut loader = loader();
    loader.insert(column);
    let mut body = Body::at(DVec3::new(8.5, 64.0, 8.5));
    body.on_ground = true;
    let game = Game::new(
        BlockRules::new(&content),
        DEFAULT_WORLD_BORDER,
        body,
        GameMode::Creative,
    );
    World {
        game,
        loader,
        content,
    }
}

pub(super) fn block(content: &Content, id: &str) -> BlockId {
    content.blocks.key(&id.parse().unwrap()).unwrap()
}

/// Puts `block`'s item in hand, then places it.
pub(super) fn place(
    w: &mut World,
    against: BlockPos,
    face: Face,
    block: BlockId,
) -> Vec<BlockChange> {
    hold(w, w.game.rules.block_item(block));
    w.game.place_block(&mut w.loader, against, face, block)
}

/// Puts one `item` in the selected slot (or empties it).
pub(super) fn hold(w: &mut World, item: Option<ItemId>) {
    let stack = item.map(|item| ItemStack::new(item, 1, &w.game.rules));
    if let Some(inventory) = w.game.ecs.inventories.get_mut(w.game.player) {
        *inventory.selected_slot_mut() = stack;
    }
}

pub(super) fn wait(world: &mut World, ticks: u32) {
    for _ in 0..ticks {
        world.game.tick(&mut world.loader);
    }
}

#[test]
fn breaking_needs_reach_and_respects_the_rate_limit() {
    let mut w = world();
    let near = BlockPos::new(9, 63, 8);
    assert!(!w.game.break_block(&mut w.loader, near).is_empty());
    let next = BlockPos::new(10, 63, 8);
    assert!(
        w.game.break_block(&mut w.loader, next).is_empty(),
        "too soon"
    );
    wait(&mut w, 2);
    assert!(!w.game.break_block(&mut w.loader, next).is_empty());
    wait(&mut w, 2);
    assert!(
        w.game
            .break_block(&mut w.loader, BlockPos::new(8, 40, 8))
            .is_empty(),
        "too far"
    );
}

#[test]
fn placing_never_fills_the_player_and_turns_logs() {
    let mut w = world();
    let log = block(&w.content, "strata:oak_log");
    let under_feet = BlockPos::new(8, 63, 8);
    assert!(place(&mut w, under_feet, Face::Up, log).is_empty());
    let beside = BlockPos::new(11, 63, 8);
    let [(pos, placed)] = place(&mut w, beside, Face::Up, log)[..] else {
        panic!("placed beside the player");
    };
    assert_eq!(pos, BlockPos::new(11, 64, 8));
    assert_eq!(placed, log);
    wait(&mut w, 2);
    let [(_, lying)] = place(&mut w, pos, Face::East, log)[..] else {
        panic!("placed against the log");
    };
    assert_eq!(lying, block(&w.content, "strata:oak_log__x"));
    wait(&mut w, 2);
    let variant = block(&w.content, "strata:oak_log__x");
    assert!(
        place(&mut w, beside, Face::West, variant).is_empty(),
        "variants have no item"
    );
}

#[test]
fn hostile_coordinates_are_refused_without_panicking() {
    let mut w = world();
    let far = BlockPos::new(i32::MAX, 10, i32::MIN);
    assert!(w.game.break_block(&mut w.loader, far).is_empty());
    assert!(place(&mut w, far, Face::East, BlockId(1)).is_empty());
}

#[test]
fn placing_while_aiming_at_grass_replaces_the_grass() {
    let mut w = world();
    let grass = block(&w.content, "strata:tall_grass");
    let planks = block(&w.content, "strata:oak_planks");
    let plant = BlockPos::new(11, 64, 8);
    assert!(w.loader.set_block(plant, grass));
    let changes = place(&mut w, plant, Face::North, planks);
    assert_eq!(changes, vec![(plant, planks)]);
    assert_eq!(w.loader.block(plant), Some(planks));
}

#[test]
fn plants_go_with_their_ground_and_need_one() {
    let mut w = world();
    let poppy = block(&w.content, "strata:poppy");
    let (ground, plant) = (BlockPos::new(11, 63, 8), BlockPos::new(11, 64, 8));
    assert!(w.loader.set_block(plant, poppy));
    let changes = w.game.break_block(&mut w.loader, ground);
    assert_eq!(changes, vec![(ground, BlockId::AIR), (plant, BlockId::AIR)]);
    assert_eq!(w.loader.block(plant), Some(BlockId::AIR));
    wait(&mut w, 2);
    assert!(w.loader.set_block(BlockPos::new(11, 62, 8), BlockId::AIR));
    let wall = BlockPos::new(10, 63, 8);
    assert!(
        place(&mut w, wall, Face::East, poppy).is_empty(),
        "no ground under the cell"
    );
}

#[test]
fn breaking_a_wall_lets_water_flow_and_the_changes_go_out() {
    let mut w = world();
    let water = block(&w.content, "strata:water");
    let (source, wall) = (BlockPos::new(12, 63, 8), BlockPos::new(11, 63, 8));
    assert!(w.loader.set_block(source, water));
    assert!(
        w.game.tick(&mut w.loader).is_empty(),
        "still until disturbed"
    );
    assert!(!w.game.break_block(&mut w.loader, wall).is_empty());
    let changes: Vec<BlockChange> = (0..10).flat_map(|_| w.game.tick(&mut w.loader)).collect();
    let flowing = block(&w.content, "strata:water__1");
    assert!(changes.contains(&(wall, flowing)));
    assert_eq!(w.loader.block(wall), Some(flowing));
}

#[test]
fn a_fire_read_back_with_its_column_burns_again() {
    let mut w = world();
    let fire = block(&w.content, "strata:fire");
    let mut column = w.loader.column(ColumnPos { x: 0, z: 0 }).unwrap().clone();
    let pos = BlockPos::new(11, 64, 8);
    column.set(11, 64, 8, fire);
    w.loader.insert(column);
    wait(&mut w, 1_000);
    assert_eq!(w.loader.block(pos), Some(BlockId::AIR), "no fuel: it died");
}

#[test]
fn torches_and_lanterns_take_the_variant_of_the_clicked_face() {
    let mut w = world();
    let cell = BlockPos::new(11, 62, 8);
    assert!(w.loader.set_block(cell, BlockId::AIR));
    let cases = [
        (
            cell.neighbour(Face::Down),
            Face::Up,
            "strata:torch",
            "strata:torch",
        ),
        (
            cell.neighbour(Face::West),
            Face::East,
            "strata:torch",
            "strata:torch__west",
        ),
        (
            cell.neighbour(Face::East),
            Face::West,
            "strata:torch",
            "strata:torch__east",
        ),
        (
            cell.neighbour(Face::North),
            Face::South,
            "strata:torch",
            "strata:torch__north",
        ),
        (
            cell.neighbour(Face::South),
            Face::North,
            "strata:torch",
            "strata:torch__south",
        ),
        (
            cell.neighbour(Face::Up),
            Face::Down,
            "strata:torch",
            "strata:torch",
        ),
        (
            cell.neighbour(Face::Up),
            Face::Down,
            "strata:lantern",
            "strata:lantern__ceiling",
        ),
        (
            cell.neighbour(Face::Down),
            Face::Up,
            "strata:lantern",
            "strata:lantern",
        ),
    ];
    for (against, face, item, expected) in cases {
        let held = block(&w.content, item);
        let changes = place(&mut w, against, face, held);
        assert_eq!(
            changes,
            vec![(cell, block(&w.content, expected))],
            "{face:?} {item}"
        );
        assert!(w.loader.set_block(cell, BlockId::AIR));
        wait(&mut w, 2);
    }
    let wall = cell.neighbour(Face::West);
    let wall_torch = block(&w.content, "strata:torch__west");
    assert!(
        place(&mut w, wall, Face::East, wall_torch).is_empty(),
        "variants have no item"
    );
    wait(&mut w, 2);
    let torch = block(&w.content, "strata:torch");
    let placed = place(&mut w, wall, Face::East, torch);
    assert_eq!(
        placed,
        vec![(cell, wall_torch)],
        "the same click with the item"
    );
}

#[test]
fn a_wall_torch_goes_with_its_wall() {
    let mut w = world();
    let torch = block(&w.content, "strata:torch");
    let wall = BlockPos::new(11, 64, 8);
    assert!(w.loader.set_block(wall, block(&w.content, "strata:stone")));
    let placed = place(&mut w, wall, Face::East, torch);
    let target = wall.neighbour(Face::East);
    assert_eq!(
        placed,
        vec![(target, block(&w.content, "strata:torch__west"))]
    );
    wait(&mut w, 2);
    let changes = w.game.break_block(&mut w.loader, wall);
    assert_eq!(changes, vec![(wall, BlockId::AIR), (target, BlockId::AIR)]);
}
