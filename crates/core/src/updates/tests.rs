#![allow(clippy::unwrap_used)]

use super::*;
use crate::content::{Content, FluidLevel};
use crate::pack::PackSet;
use std::path::Path;
use std::sync::OnceLock;

pub(super) const GROUND: i32 = 64;

pub(super) fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        Content::load(&packs, &PackSet::default()).0
    })
}

pub(super) fn id(name: &str) -> BlockId {
    content().blocks.key(&name.parse().unwrap()).unwrap()
}

/// Endless flat stone below y = 64, air above; columns beyond `loaded` are not loaded.
pub(super) struct Plane {
    pub(super) blocks: HashMap<BlockPos, BlockId>,
    stone: BlockId,
    loaded: i32,
}

impl Plane {
    pub(super) fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            stone: id("strata:stone"),
            loaded: i32::MAX,
        }
    }

    pub(super) fn put(&mut self, pos: BlockPos, block: BlockId) {
        self.blocks.insert(pos, block);
    }

    fn fluid_cells(&self) -> Vec<BlockPos> {
        let rules = UpdateRules::new(content());
        let mut cells: Vec<BlockPos> = self
            .blocks
            .iter()
            .filter(|(_, b)| rules.fluid_state(**b).is_some())
            .map(|(pos, _)| *pos)
            .collect();
        cells.sort_by_key(|p| (p.x, p.y, p.z));
        cells
    }
}

impl BlockAccess for Plane {
    fn block(&self, pos: BlockPos) -> Option<BlockId> {
        if pos.x.abs() >= self.loaded || pos.z.abs() >= self.loaded {
            return None;
        }
        let default = if pos.y < GROUND {
            self.stone
        } else {
            BlockId::AIR
        };
        Some(self.blocks.get(&pos).copied().unwrap_or(default))
    }

    fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        let loaded = self.block(pos).is_some();
        if loaded {
            self.blocks.insert(pos, block);
        }
        loaded
    }
}

pub(super) fn updates() -> BlockUpdates {
    BlockUpdates::new(
        Arc::new(UpdateRules::new(content())),
        FluidSettings::default(),
    )
}

/// Ticks until no block is active; returns the ticks taken and every change.
fn settle(sim: &mut BlockUpdates, world: &mut Plane, limit: u32) -> (u32, Vec<BlockChange>) {
    let mut changes = Vec::new();
    for tick in 0..limit {
        if sim.active() == 0 {
            return (tick, changes);
        }
        sim.tick(world, &mut changes);
    }
    panic!("still {} active blocks after {limit} ticks", sim.active());
}

fn level(world: &Plane, pos: BlockPos) -> Option<FluidLevel> {
    let block = world.block(pos)?;
    UpdateRules::new(content())
        .fluid_state(block)
        .map(|s| s.level)
}

pub(super) fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos::new(x, y, z)
}

#[test]
fn every_fluid_has_its_variants_and_name() {
    let water = content().fluids.get(&id("strata:water")).unwrap();
    assert_eq!(water.flowing.len(), 7);
    assert_eq!(id("strata:water__7"), water.flowing[6]);
    assert_eq!(id("strata:water__falling"), water.falling);
    let lava = content().fluids.get(&id("strata:lava")).unwrap();
    assert_eq!(lava.flowing.len(), 3);
    assert_eq!(lava.hardens.unwrap().source, id("strata:obsidian"));
    let variant = content().blocks.get(water.flowing[2]).unwrap();
    assert!(!variant.has_item && variant.category.is_none());
    let name = content().block_name(water.falling).unwrap();
    assert_eq!(name.to_string(), "strata:water");
}

#[test]
fn a_source_in_a_closed_hole_settles() {
    let mut world = Plane::new();
    for (x, y, z) in (0..5).flat_map(|x| (62..64).flat_map(move |y| (0..5).map(move |z| (x, y, z))))
    {
        world.put(at(x, y, z), BlockId::AIR);
    }
    let mut sim = updates();
    let mut changes = Vec::new();
    assert!(sim.set_block(&mut world, at(2, 63, 2), id("strata:water"), &mut changes));
    settle(&mut sim, &mut world, 2000);
    let cells = world.fluid_cells();
    assert_eq!(
        cells.len(),
        30,
        "the whole bottom, the source and its 4 sides"
    );
    assert!(
        cells
            .iter()
            .all(|p| (0..5).contains(&p.x) && (62..64).contains(&p.y))
    );
    let mut more = Vec::new();
    for _ in 0..200 {
        sim.tick(&mut world, &mut more);
    }
    assert!(more.is_empty(), "nothing moves once settled");
    assert_eq!(world.fluid_cells(), cells);
}

#[test]
fn a_source_on_an_endless_plane_stops_at_its_reach() {
    let mut world = Plane::new();
    let mut sim = updates();
    sim.set_block(
        &mut world,
        at(0, GROUND, 0),
        id("strata:water"),
        &mut Vec::new(),
    );
    settle(&mut sim, &mut world, 2000);
    let cells = world.fluid_cells();
    assert_eq!(cells.len(), 1 + 2 * 7 * 8, "a diamond of radius 7");
    assert!(
        cells
            .iter()
            .all(|p| p.x.abs() + p.z.abs() <= 7 && p.y == GROUND)
    );
    assert_eq!(
        level(&world, at(3, GROUND, 4)),
        Some(FluidLevel::Flowing(7))
    );
}

#[test]
fn liquids_flow_towards_the_nearest_way_down() {
    let mut world = Plane::new();
    world.put(at(3, GROUND - 1, 0), BlockId::AIR);
    let mut sim = updates();
    sim.set_block(
        &mut world,
        at(0, GROUND, 0),
        id("strata:water"),
        &mut Vec::new(),
    );
    let mut changes = Vec::new();
    for _ in 0..6 {
        sim.tick(&mut world, &mut changes);
    }
    assert_eq!(changes.len(), 1, "one side only: towards the hole");
    assert_eq!(changes[0].0, at(1, GROUND, 0));
}

#[test]
fn breaking_a_lake_wall_lets_the_water_out() {
    let mut world = Plane::new();
    let water = id("strata:water");
    for (x, z) in (-1..5).flat_map(|x| (-1..5).map(move |z| (x, z))) {
        let inside = (0..4).contains(&x) && (0..4).contains(&z);
        world.put(at(x, GROUND, z), if inside { water } else { world.stone });
    }
    let mut sim = updates();
    settle(&mut sim, &mut world, 10);
    assert_eq!(sim.active(), 0, "generated water is still");
    sim.set_block(&mut world, at(4, GROUND, 1), BlockId::AIR, &mut Vec::new());
    settle(&mut sim, &mut world, 2000);
    assert_eq!(
        level(&world, at(4, GROUND, 1)),
        Some(FluidLevel::Flowing(1))
    );
    assert_eq!(
        level(&world, at(10, GROUND, 1)),
        Some(FluidLevel::Flowing(7))
    );
    assert_eq!(level(&world, at(11, GROUND, 1)), None);
}

#[test]
fn removing_the_source_dries_the_flow() {
    let mut world = Plane::new();
    let mut sim = updates();
    sim.set_block(
        &mut world,
        at(0, GROUND + 3, 0),
        id("strata:water"),
        &mut Vec::new(),
    );
    settle(&mut sim, &mut world, 2000);
    assert_eq!(level(&world, at(0, GROUND, 0)), Some(FluidLevel::Falling));
    sim.set_block(
        &mut world,
        at(0, GROUND + 3, 0),
        BlockId::AIR,
        &mut Vec::new(),
    );
    settle(&mut sim, &mut world, 2000);
    assert!(world.fluid_cells().is_empty());
}

#[test]
fn water_and_lava_make_stone_or_obsidian() {
    let mut world = Plane::new();
    let mut sim = updates();
    let (lava, water) = (id("strata:lava"), id("strata:water"));
    sim.set_block(&mut world, at(0, GROUND, 0), lava, &mut Vec::new());
    settle(&mut sim, &mut world, 2000);
    assert_eq!(
        level(&world, at(2, GROUND, 0)),
        Some(FluidLevel::Flowing(2))
    );
    sim.set_block(&mut world, at(2, GROUND + 1, 0), water, &mut Vec::new());
    sim.set_block(&mut world, at(0, GROUND + 1, 0), water, &mut Vec::new());
    for _ in 0..40 {
        sim.tick(&mut world, &mut Vec::new());
    }
    assert_eq!(world.block(at(2, GROUND, 0)), Some(id("strata:stone")));
    assert_eq!(world.block(at(0, GROUND, 0)), Some(id("strata:obsidian")));
}

#[test]
fn falling_lava_hardens_water_below() {
    let mut world = Plane::new();
    let mut sim = updates();
    world.put(at(0, GROUND, 0), id("strata:water"));
    sim.set_block(
        &mut world,
        at(0, GROUND + 2, 0),
        id("strata:lava"),
        &mut Vec::new(),
    );
    for _ in 0..100 {
        sim.tick(&mut world, &mut Vec::new());
    }
    assert_eq!(world.block(at(0, GROUND, 0)), Some(id("strata:stone")));
}

#[test]
fn water_between_two_sources_becomes_a_source() {
    let mut world = Plane::new();
    let mut sim = updates();
    let water = id("strata:water");
    sim.set_block(&mut world, at(0, GROUND, 0), water, &mut Vec::new());
    sim.set_block(&mut world, at(2, GROUND, 0), water, &mut Vec::new());
    settle(&mut sim, &mut world, 2000);
    assert_eq!(level(&world, at(1, GROUND, 0)), Some(FluidLevel::Source));
}

#[test]
fn the_flow_is_deterministic_whatever_the_budget() {
    let run = |budget: u32| {
        let mut world = Plane::new();
        world.put(at(2, GROUND - 1, 3), BlockId::AIR);
        let settings = FluidSettings {
            flow: true,
            updates_per_tick: budget,
        };
        let mut sim = BlockUpdates::new(Arc::new(UpdateRules::new(content())), settings);
        sim.set_block(
            &mut world,
            at(0, GROUND, 0),
            id("strata:water"),
            &mut Vec::new(),
        );
        sim.set_block(
            &mut world,
            at(6, GROUND, 1),
            id("strata:lava"),
            &mut Vec::new(),
        );
        let (_, changes) = settle(&mut sim, &mut world, 20_000);
        (changes, world.fluid_cells())
    };
    assert_eq!(run(2048), run(2048));
    assert_eq!(
        run(1),
        run(1),
        "a tiny budget is slower, still deterministic"
    );
}

#[test]
fn a_liquid_waits_at_the_edge_of_unloaded_columns() {
    let mut world = Plane::new();
    world.loaded = 3;
    let mut sim = updates();
    sim.set_block(
        &mut world,
        at(0, GROUND, 0),
        id("strata:water"),
        &mut Vec::new(),
    );
    for _ in 0..400 {
        sim.tick(&mut world, &mut Vec::new());
    }
    assert!(sim.active() > 0, "the edge keeps waiting");
    world.loaded = i32::MAX;
    settle(&mut sim, &mut world, 2000);
    assert_eq!(world.fluid_cells().len(), 113);
}

#[test]
fn frozen_liquids_do_not_flow() {
    let mut world = Plane::new();
    let mut sim = updates();
    sim.set_settings(FluidSettings {
        flow: false,
        ..FluidSettings::default()
    });
    sim.set_block(
        &mut world,
        at(0, GROUND, 0),
        id("strata:water"),
        &mut Vec::new(),
    );
    assert_eq!(sim.active(), 0);
    sim.tick(&mut world, &mut Vec::new());
    assert_eq!(world.fluid_cells().len(), 1);
}

#[test]
fn plants_and_vines_drop_without_support() {
    let mut world = Plane::new();
    let mut sim = updates();
    let (poppy, vine, leaves) = (
        id("strata:poppy"),
        id("strata:vine"),
        id("strata:oak_leaves"),
    );
    world.put(at(0, GROUND, 0), poppy);
    world.put(at(5, GROUND + 5, 0), leaves);
    for y in GROUND + 2..GROUND + 5 {
        world.put(at(5, y, 0), vine);
    }
    let mut changes = Vec::new();
    sim.set_block(&mut world, at(0, GROUND - 1, 0), BlockId::AIR, &mut changes);
    assert_eq!(
        changes,
        vec![
            (at(0, GROUND - 1, 0), BlockId::AIR),
            (at(0, GROUND, 0), BlockId::AIR)
        ]
    );
    changes.clear();
    sim.set_block(&mut world, at(5, GROUND + 5, 0), BlockId::AIR, &mut changes);
    assert_eq!(changes.len(), 4, "the leaves and the whole vine");
    assert!((GROUND + 2..GROUND + 5).all(|y| world.block(at(5, y, 0)) == Some(BlockId::AIR)));
}

#[test]
fn water_washes_plants_away() {
    let mut world = Plane::new();
    world.put(at(1, GROUND, 0), id("strata:tall_grass"));
    let mut sim = updates();
    sim.set_block(
        &mut world,
        at(0, GROUND, 0),
        id("strata:water"),
        &mut Vec::new(),
    );
    settle(&mut sim, &mut world, 2000);
    assert_eq!(
        level(&world, at(1, GROUND, 0)),
        Some(FluidLevel::Flowing(1))
    );
}

/// A lake pours through a breach 32 blocks wide down a cliff into a pit (timed by the
/// `fluids` benchmark).
#[test]
fn a_large_flow_settles() {
    let mut world = Plane::new();
    let water = id("strata:water");
    for (x, z) in (0..32).flat_map(|x| (0..32).map(move |z| (x, z))) {
        world.put(at(x, GROUND, z), water);
    }
    for (x, z) in (-20..0).flat_map(|x| (-8..40).map(move |z| (x, z))) {
        for y in GROUND - 6..GROUND {
            world.put(at(x, y, z), BlockId::AIR);
        }
    }
    let mut sim = updates();
    let mut changes = Vec::new();
    for z in 0..32 {
        world.put(at(-1, GROUND, z), world.stone);
        sim.set_block(&mut world, at(-1, GROUND, z), BlockId::AIR, &mut changes);
    }
    let (_, flowed) = settle(&mut sim, &mut world, 5000);
    assert!(changes.len() + flowed.len() > 1000);
}
