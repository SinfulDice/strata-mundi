#![allow(clippy::unwrap_used)]

use super::tests::{GROUND, Plane, at, id, updates};
use super::*;
use std::collections::HashSet;

/// Trunks every other block on a 9 × 9 square, under a roof of leaves two blocks thick.
fn forest() -> Plane {
    let mut world = Plane::new();
    let (log, leaves) = (id("strata:oak_log"), id("strata:oak_leaves"));
    for x in -4..=4 {
        for z in -4..=4 {
            let trunk = x % 2 == 0 && z % 2 == 0;
            for y in GROUND..GROUND + 4 {
                if trunk {
                    world.put(at(x, y, z), log);
                } else if y >= GROUND + 2 {
                    world.put(at(x, y, z), leaves);
                }
            }
        }
    }
    world
}

fn fire() -> BlockId {
    id("strata:fire")
}

/// Ticks until nothing is planned (panics after `limit`); returns every change.
fn burn_out(sim: &mut BlockUpdates, world: &mut impl BlockAccess, limit: u32) -> Vec<BlockChange> {
    let mut changes = Vec::new();
    for _ in 0..limit {
        if sim.active() == 0 {
            return changes;
        }
        sim.tick(world, &mut changes);
    }
    panic!("still {} active blocks after {limit} ticks", sim.active());
}

fn lit(changes: &[BlockChange]) -> HashSet<BlockPos> {
    changes
        .iter()
        .filter(|(_, block)| *block == fire())
        .map(|(pos, _)| *pos)
        .collect()
}

fn light(sim: &mut BlockUpdates, world: &mut Plane) -> Vec<BlockChange> {
    let mut changes = Vec::new();
    assert!(sim.set_block(world, at(1, GROUND, 0), fire(), &mut changes));
    changes
}

fn burnable_left(world: &Plane) -> usize {
    let rules = UpdateRules::new(super::tests::content());
    world
        .blocks
        .values()
        .filter(|&&block| rules.flammability(Cell::Block(block)).is_some())
        .count()
}

#[test]
fn a_forest_fire_spreads_then_dies_out_within_the_forest() {
    let mut world = forest();
    let before = burnable_left(&world);
    let mut sim = updates();
    let mut changes = light(&mut sim, &mut world);
    changes.extend(burn_out(&mut sim, &mut world, 200_000));
    let lit = lit(&changes);
    assert!(lit.len() > 20, "the fire spread to {} blocks", lit.len());
    for pos in &lit {
        let inside = pos.x.abs() <= 5 && pos.z.abs() <= 5 && (GROUND..=GROUND + 8).contains(&pos.y);
        assert!(inside, "fire escaped the forest at {pos:?}");
    }
    assert!(world.blocks.values().all(|&block| block != fire()));
    assert!(
        burnable_left(&world) + 20 < before,
        "part of the forest burned away"
    );
    assert!(sim.fire_ages.is_empty());
}

#[test]
fn without_spread_no_new_fire_is_lit() {
    let mut world = forest();
    let mut sim = updates();
    sim.set_fire_settings(FireSettings {
        spread: false,
        destroy: true,
    });
    let mut changes = light(&mut sim, &mut world);
    changes.extend(burn_out(&mut sim, &mut world, 20_000));
    assert_eq!(lit(&changes), HashSet::from([at(1, GROUND, 0)]));
}

#[test]
fn without_destroy_nothing_burns_away() {
    let mut world = forest();
    let before = burnable_left(&world);
    let mut sim = updates();
    sim.set_fire_settings(FireSettings {
        spread: true,
        destroy: false,
    });
    let mut changes = light(&mut sim, &mut world);
    changes.extend(burn_out(&mut sim, &mut world, 200_000));
    assert!(lit(&changes).len() > 1, "it still spreads");
    assert_eq!(burnable_left(&world), before);
}

#[test]
fn water_puts_fire_out_and_keeps_it_away() {
    let mut world = forest();
    let mut sim = updates();
    light(&mut sim, &mut world);
    let mut changes = Vec::new();
    sim.set_block(
        &mut world,
        at(1, GROUND, 1),
        id("strata:water"),
        &mut changes,
    );
    assert_eq!(world.block(at(1, GROUND, 0)), Some(BlockId::AIR));
    assert!(changes.contains(&(at(1, GROUND, 0), BlockId::AIR)));
    assert!(!sim.fire_can_stay(&world, at(1, GROUND, 0)).unwrap());
}

#[test]
fn fire_needs_ground_or_fuel() {
    let world = Plane::new();
    let sim = updates();
    assert_eq!(sim.fire_can_stay(&world, at(0, GROUND, 0)), Some(true));
    assert_eq!(sim.fire_can_stay(&world, at(0, GROUND + 3, 0)), Some(false));
    let mut changes = Vec::new();
    let mut world = world;
    let mut sim = sim;
    sim.set_block(&mut world, at(0, GROUND, 0), fire(), &mut changes);
    burn_out(&mut sim, &mut world, 2_000);
    assert_eq!(world.block(at(0, GROUND, 0)), Some(BlockId::AIR), "no fuel");
}

#[test]
fn a_fire_burns_the_same_way_every_time_at_equal_budget() {
    let run = |budget| {
        let mut world = forest();
        let mut sim = updates();
        sim.set_settings(FluidSettings {
            updates_per_tick: budget,
            ..FluidSettings::default()
        });
        let mut changes = light(&mut sim, &mut world);
        changes.extend(burn_out(&mut sim, &mut world, 400_000));
        changes
    };
    assert_eq!(run(2048), run(2048));
    assert_eq!(run(2), run(2));
}

fn digest(changes: &[BlockChange]) -> u64 {
    changes
        .iter()
        .fold(0xCBF2_9CE4_8422_2325, |hash, (pos, block)| {
            let value = [pos.x, pos.y, pos.z, i32::from(block.0)];
            value.iter().fold(hash, |hash, &v| {
                (hash ^ u64::from(v as u32)).wrapping_mul(0x0100_0000_01B3)
            })
        })
}

fn forest_fire(seed: u64, dimension: &str) -> Vec<BlockChange> {
    let mut world = forest();
    let mut sim = updates();
    sim.set_seed(
        seed,
        &crate::dimension::DimensionId(dimension.parse().unwrap()),
    );
    let mut changes = light(&mut sim, &mut world);
    changes.extend(burn_out(&mut sim, &mut world, 200_000));
    changes
}

/// Pinned so that a change of the rules or the hash, which would make saved worlds
/// burn differently, is noticed. The digest holds block ids: a new block of the pack
/// moves it while the count stays.
#[test]
fn a_forest_fire_matches_its_reference() {
    let changes = forest_fire(1, "strata:terra");
    assert_eq!(
        (changes.len(), digest(&changes)),
        (863, 16_244_730_914_320_168_529)
    );
}

#[test]
fn the_world_seed_and_dimension_change_the_fire() {
    let reference = forest_fire(1, "strata:terra");
    assert_ne!(reference, forest_fire(2, "strata:terra"));
    assert_ne!(reference, forest_fire(1, "strata:other"));
}

/// Rain everywhere: the entry point of the weather (v0.5).
struct Rainy(Plane);

impl BlockAccess for Rainy {
    fn block(&self, pos: BlockPos) -> Option<BlockId> {
        self.0.block(pos)
    }
    fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        self.0.set_block(pos, block)
    }
    fn rains_on(&self, _pos: BlockPos) -> bool {
        true
    }
}

#[test]
fn rain_puts_fire_out() {
    let mut world = Rainy(forest());
    let mut sim = updates();
    let mut changes = Vec::new();
    sim.set_block(&mut world, at(1, GROUND, 0), fire(), &mut changes);
    burn_out(&mut sim, &mut world, 100);
    assert_eq!(world.block(at(1, GROUND, 0)), Some(BlockId::AIR));
}

#[test]
fn a_fire_step_weighs_three_liquid_steps_in_the_budget() {
    let stepped = |budget| {
        let mut world = Plane::new();
        let mut sim = updates();
        sim.set_settings(FluidSettings {
            updates_per_tick: budget,
            ..FluidSettings::default()
        });
        for x in 0..30 {
            sim.set_block(&mut world, at(x * 3, GROUND, 0), fire(), &mut Vec::new());
        }
        for _ in 0..40 {
            sim.tick(&mut world, &mut Vec::new());
        }
        sim.fire_ages.len()
    };
    assert_eq!(stepped(90), 30);
    assert_eq!(stepped(3), 11, "one fire step in each of ticks 30 to 40");
}
