#![allow(clippy::expect_used)]

//! Heat bands: biomes more than one band apart (desert and snow) never touch.

mod common;

use std::sync::Arc;
use strata_worldgen::{Biome, Generator, Seed, WorldgenContent};

const SEEDS: [u64; 3] = [42, 7, 99];
/// 6 144 blocks of side, sampled every 4 blocks (4 096 in debug builds, which are slower).
const REFERENCE: Grid = Grid {
    side: if cfg!(debug_assertions) { 1024 } else { 1536 },
    step: 4,
};

fn biome<'a>(worldgen: &'a WorldgenContent, name: &str) -> &'a Biome {
    worldgen
        .biomes()
        .iter()
        .find(|b| b.id.name() == name)
        .expect("pack biome")
}

#[test]
fn the_pack_keeps_hot_and_dry_biomes_away_from_snow() {
    let (_, worldgen) = common::load();
    for hot in ["desert", "badlands"] {
        for cold in ["snowy_tundra", "snowy_peaks", "taiga"] {
            let (hot, cold) = (biome(&worldgen, hot), biome(&worldgen, cold));
            assert!(!hot.may_touch(cold), "{} touches {}", hot.id, cold.id);
        }
    }
    let plains = biome(&worldgen, "plains");
    assert!(plains.may_touch(biome(&worldgen, "desert")));
    assert!(plains.may_touch(biome(&worldgen, "taiga")));
    assert!(biome(&worldgen, "ocean").may_touch(biome(&worldgen, "desert")));
}

#[test]
fn heat_bands_split_temperatures_in_order() {
    let (_, worldgen) = common::load();
    let terrain = &worldgen.terrain;
    let bands: Vec<u8> = [-1.0, -0.2, 0.0, 0.2, 1.0]
        .into_iter()
        .map(|t| terrain.heat_band(t))
        .collect();
    assert!(bands.windows(2).all(|pair| pair[0] <= pair[1]), "{bands:?}");
    assert_eq!(bands[0], 0);
    assert_eq!(usize::from(bands[4]), terrain.heat_bands.len());
}

/// A square of `side` × `side` samples `step` blocks apart, centred on the origin.
#[derive(Clone, Copy)]
struct Grid {
    side: i32,
    step: i32,
}

impl Grid {
    fn point(self, i: i32) -> (i32, i32) {
        let half = self.side / 2;
        (
            (i % self.side - half) * self.step,
            (i / self.side - half) * self.step,
        )
    }

    /// Biome indices, row by row.
    fn biomes(self, generator: &Generator) -> Vec<usize> {
        (0..self.side * self.side)
            .map(|i| self.point(i))
            .map(|(x, z)| usize::from(generator.sample(x, z).1.index.0))
            .collect()
    }

    /// Every 97th point of `biomes`, sampled backwards: the order must not matter.
    fn in_reverse(self, generator: &Generator) -> impl Iterator<Item = usize> + '_ {
        (0..self.side * self.side)
            .step_by(97)
            .rev()
            .map(move |i| self.point(i))
            .map(|(x, z)| usize::from(generator.sample(x, z).1.index.0))
    }
}

fn forbidden_contacts(biomes: &[Biome], grid: &[usize], side: usize) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for row in 0..side {
        for col in 0..side {
            let here = &biomes[grid[row * side + col]];
            let east = (col + 1 < side).then(|| grid[row * side + col + 1]);
            let south = (row + 1 < side).then(|| grid[(row + 1) * side + col]);
            for there in [east, south].into_iter().flatten().map(|i| &biomes[i]) {
                if !here.may_touch(there) {
                    found.push((here.id.to_string(), there.id.to_string()));
                }
            }
        }
    }
    found
}

fn check_borders(worldgen: &Arc<WorldgenContent>, seed: u64, area: Grid) {
    let generator = Generator::new(Seed(seed), worldgen.clone());
    let grid = area.biomes(&generator);
    let biomes = worldgen.biomes();
    let side = usize::try_from(area.side).expect("positive side");
    let contacts = forbidden_contacts(biomes, &grid, side);
    assert!(contacts.is_empty(), "seed {seed}: {:?}", &contacts[..1]);
    let extreme = |band: u8| {
        grid.iter()
            .any(|&i| biomes[i].heat.is_some_and(|(a, b)| a == band && b == band))
    };
    let hottest = u8::try_from(worldgen.terrain.heat_bands.len()).expect("few bands");
    assert!(
        extreme(0) && extreme(hottest),
        "seed {seed}: no hot or no cold biome"
    );
    let again = Generator::new(Seed(seed), worldgen.clone());
    assert!(
        area.in_reverse(&again)
            .eq(grid.iter().step_by(97).rev().copied()),
        "seed {seed}: biomes changed"
    );
}

#[test]
fn no_forbidden_neighbours_on_reference_seeds() {
    let (_, worldgen) = common::load();
    std::thread::scope(|scope| {
        for seed in SEEDS {
            let worldgen = &worldgen;
            scope.spawn(move || check_borders(worldgen, seed, REFERENCE));
        }
    });
}

/// The smallest biome size of the slider: the bands must still be wider than what the
/// temperature changes from one block to the next.
#[test]
fn no_forbidden_neighbours_with_the_smallest_biomes() {
    let (_, worldgen) = common::load();
    let mut small = (*worldgen).clone();
    small.terrain.biome_scale = 0.25;
    let area = Grid { side: 768, step: 2 };
    check_borders(&Arc::new(small), 42, area);
}
