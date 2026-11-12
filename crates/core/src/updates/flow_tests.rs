//! How water chooses its way, as in Minecraft: towards the nearest way down within
//! `slope_search` blocks, all four sides only when there is none.

#![allow(clippy::unwrap_used)]

use super::tests::{GROUND, Plane, at, content, id, level, settle, updates};
use super::*;
use crate::content::FluidLevel;

fn pour(world: &mut Plane, pos: BlockPos) -> BlockUpdates {
    let mut sim = updates();
    sim.set_block(world, pos, id("strata:water"), &mut Vec::new());
    sim
}

/// Water poured at the origin of a plateau with a hole `distance` blocks east, settled.
fn plateau_with_hole(distance: i32) -> Plane {
    let mut world = Plane::new();
    world.put(at(distance, GROUND - 1, 0), BlockId::AIR);
    let mut sim = pour(&mut world, at(0, GROUND, 0));
    settle(&mut sim, &mut world, 2000);
    world
}

fn assert_only_towards_the_hole(distance: i32) {
    let world = plateau_with_hole(distance);
    let mut expected: Vec<BlockPos> = (0..=distance).map(|x| at(x, GROUND, 0)).collect();
    expected.push(at(distance, GROUND - 1, 0));
    expected.sort_by_key(|p| (p.x, p.y, p.z));
    assert_eq!(
        world.fluid_cells(),
        expected,
        "hole {distance} blocks away: a single line to it, nothing beside it"
    );
    assert_eq!(
        level(&world, at(distance, GROUND - 1, 0)),
        Some(FluidLevel::Falling)
    );
}

#[test]
fn on_a_plateau_water_only_goes_to_a_hole_three_blocks_away() {
    assert_only_towards_the_hole(3);
}

/// `slope_search` (4) counts from the side next to the water: the search sees holes up
/// to five blocks from it, as in Minecraft.
#[test]
fn the_search_reaches_holes_four_and_five_blocks_away() {
    assert_only_towards_the_hole(4);
    assert_only_towards_the_hole(5);
}

#[test]
fn without_a_hole_within_reach_of_the_search_water_widens() {
    let mut world = Plane::new();
    world.put(at(6, GROUND - 1, 0), BlockId::AIR);
    let mut sim = pour(&mut world, at(0, GROUND, 0));
    let water = content().fluids.get(&id("strata:water")).unwrap();
    let mut changes = Vec::new();
    for _ in 0..water.tick_rate {
        sim.tick(&mut world, &mut changes);
    }
    let mut first: Vec<BlockPos> = changes.iter().map(|(pos, _)| *pos).collect();
    first.sort_by_key(|p| (p.x, p.z));
    assert_eq!(
        first,
        vec![
            at(-1, GROUND, 0),
            at(0, GROUND, -1),
            at(0, GROUND, 1),
            at(1, GROUND, 0)
        ]
    );
    settle(&mut sim, &mut world, 2000);
    assert_eq!(
        level(&world, at(-7, GROUND, 0)),
        Some(FluidLevel::Flowing(7))
    );
    assert_eq!(
        level(&world, at(0, GROUND, 7)),
        Some(FluidLevel::Flowing(7))
    );
}

/// Steps one block lower every four blocks: the hole of each step is always in sight.
fn staircase() -> Plane {
    let mut world = Plane::new();
    for x in 0..40 {
        let surface = GROUND - x / 4;
        for z in -20..=20 {
            for y in surface..GROUND {
                world.put(at(x, y, z), BlockId::AIR);
            }
        }
    }
    world
}

#[test]
fn a_flow_down_a_staircase_stays_a_narrow_stream_and_settles() {
    let mut world = staircase();
    let mut sim = pour(&mut world, at(0, GROUND, 0));
    settle(&mut sim, &mut world, 4000);
    let cells = world.fluid_cells();
    let wide = cells.iter().filter(|p| p.x < 36 && p.z != 0).count();
    assert_eq!(wide, 0, "each step leads straight to the next hole");
    let in_the_stream_or_the_last_diamond = |p: &BlockPos| p.z == 0 || p.x - p.z.abs() >= 29;
    assert!(cells.iter().all(in_the_stream_or_the_last_diamond));
    let mut more = Vec::new();
    for _ in 0..200 {
        sim.tick(&mut world, &mut more);
    }
    assert!(more.is_empty(), "nothing moves once settled");
}

#[test]
fn a_falling_flow_is_full_again_at_the_bottom() {
    let mut world = staircase();
    let mut sim = pour(&mut world, at(0, GROUND, 0));
    settle(&mut sim, &mut world, 4000);
    assert_eq!(
        level(&world, at(4, GROUND - 1, 0)),
        Some(FluidLevel::Falling)
    );
    assert_eq!(
        level(&world, at(5, GROUND - 1, 0)),
        Some(FluidLevel::Flowing(1))
    );
}

#[test]
fn two_sources_above_a_source_make_a_source() {
    let mut world = Plane::new();
    let water = id("strata:water");
    world.put(at(1, GROUND - 1, 0), water);
    world.put(at(0, GROUND - 1, 0), world.stone);
    world.put(at(2, GROUND - 1, 0), world.stone);
    let mut sim = pour(&mut world, at(0, GROUND, 0));
    sim.set_block(&mut world, at(2, GROUND, 0), water, &mut Vec::new());
    settle(&mut sim, &mut world, 2000);
    assert_eq!(level(&world, at(1, GROUND, 0)), Some(FluidLevel::Source));
}
