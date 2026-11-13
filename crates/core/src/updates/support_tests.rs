#![allow(clippy::unwrap_used)]

use super::tests::{GROUND, Plane, at, content, id, updates};
use super::*;

fn variant(world: &Plane, block: &str, clicked: Face, target: BlockPos) -> Option<BlockId> {
    UpdateRules::new(content()).placed_variant(id(block), clicked, |side| {
        cell(world, target.neighbour(side))
    })
}

/// A stone pillar at x = 0 from the ground to y = GROUND + 3, under a stone slab.
fn pillar() -> Plane {
    let mut world = Plane::new();
    for y in GROUND..GROUND + 4 {
        world.put(at(0, y, 0), world.stone);
    }
    world
}

#[test]
fn torches_lean_on_the_clicked_face() {
    let world = pillar();
    let wall = at(0, GROUND + 1, 0);
    for (clicked, name) in [
        (Face::North, "strata:torch__south"),
        (Face::South, "strata:torch__north"),
        (Face::East, "strata:torch__west"),
        (Face::West, "strata:torch__east"),
    ] {
        let target = wall.neighbour(clicked);
        assert_eq!(
            variant(&world, "strata:torch", clicked, target),
            Some(id(name))
        );
    }
    let on_top = at(0, GROUND + 4, 0);
    let floor = Some(id("strata:torch"));
    assert_eq!(variant(&world, "strata:torch", Face::Up, on_top), floor);
}

#[test]
fn lanterns_stand_or_hang() {
    let mut world = pillar();
    world.put(at(1, GROUND + 3, 0), world.stone);
    let under = at(1, GROUND + 2, 0);
    let hanging = Some(id("strata:lantern__ceiling"));
    assert_eq!(
        variant(&world, "strata:lantern", Face::Down, under),
        hanging
    );
    let ground = at(2, GROUND, 0);
    let standing = Some(id("strata:lantern"));
    assert_eq!(
        variant(&world, "strata:lantern", Face::Up, ground),
        standing
    );
    let beside = at(1, GROUND + 1, 0);
    assert_eq!(
        variant(&world, "strata:lantern", Face::East, beside),
        None,
        "no wall variant, no floor nor ceiling"
    );
    assert_eq!(
        variant(&world, "strata:lantern", Face::East, at(1, GROUND, 0)),
        standing,
        "falls back to the floor below"
    );
}

#[test]
fn a_torch_needs_a_support() {
    let world = Plane::new();
    let in_the_air = at(0, GROUND + 3, 0);
    assert_eq!(
        variant(&world, "strata:torch", Face::Down, in_the_air),
        None
    );
    let mut edge = Plane::new();
    edge.loaded = 1;
    assert_eq!(
        variant(&edge, "strata:torch", Face::West, at(0, GROUND, 0)),
        None,
        "the clicked wall is not loaded: no guess on another side"
    );
    assert_eq!(
        variant(&world, "strata:torch", Face::Down, at(0, GROUND, 0)),
        Some(id("strata:torch")),
        "no ceiling variant: stands on the floor instead"
    );
}

#[test]
fn torches_and_lanterns_drop_with_their_support() {
    let mut world = pillar();
    let mut sim = updates();
    let wall_torch = at(1, GROUND + 1, 0);
    let hanging = at(-1, GROUND + 2, 0);
    let standing = at(0, GROUND + 4, 0);
    world.put(at(-1, GROUND + 3, 0), world.stone);
    world.put(wall_torch, id("strata:torch__west"));
    world.put(hanging, id("strata:lantern__ceiling"));
    world.put(standing, id("strata:lantern"));
    let mut changes = Vec::new();
    sim.set_block(&mut world, at(0, GROUND + 2, 0), BlockId::AIR, &mut changes);
    assert_eq!(
        world.block(wall_torch),
        Some(id("strata:torch__west")),
        "not its wall"
    );
    sim.set_block(&mut world, at(0, GROUND + 1, 0), BlockId::AIR, &mut changes);
    assert_eq!(world.block(wall_torch), Some(BlockId::AIR));
    sim.set_block(
        &mut world,
        at(-1, GROUND + 3, 0),
        BlockId::AIR,
        &mut changes,
    );
    assert_eq!(world.block(hanging), Some(BlockId::AIR));
    sim.set_block(&mut world, at(0, GROUND + 3, 0), BlockId::AIR, &mut changes);
    assert_eq!(world.block(standing), Some(BlockId::AIR));
}
