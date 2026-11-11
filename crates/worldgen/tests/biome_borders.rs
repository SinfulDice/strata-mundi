#![allow(clippy::expect_used)]

//! Heat bands: biomes more than one band apart (desert and snow) never touch.

mod common;

use strata_worldgen::{Biome, Generator, WorldgenContent};

const SEEDS: [u64; 3] = [42, 7, 99];
/// Blocks between two samples, and samples per side.
const STEP: i32 = 4;
const SIDE: i32 = 1536;

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

/// Biome indices of a square grid centred on the origin, row by row.
fn biome_grid(generator: &Generator) -> Vec<usize> {
    let half = SIDE / 2;
    (-half..half)
        .flat_map(|gz| (-half..half).map(move |gx| (gx * STEP, gz * STEP)))
        .map(|(x, z)| usize::from(generator.sample(x, z).1.index.0))
        .collect()
}

/// Every 97th point of `biome_grid`, sampled backwards: the order must not matter.
fn grid_in_reverse(generator: &Generator) -> impl Iterator<Item = usize> + '_ {
    let half = SIDE / 2;
    let count = SIDE * SIDE;
    (0..count)
        .step_by(97)
        .rev()
        .map(move |i| ((i % SIDE - half) * STEP, (i / SIDE - half) * STEP))
        .map(|(x, z)| usize::from(generator.sample(x, z).1.index.0))
}

fn forbidden_contacts(biomes: &[Biome], grid: &[usize]) -> Vec<(String, String)> {
    let side = SIDE as usize;
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

#[test]
fn no_forbidden_neighbours_on_reference_seeds() {
    let (_, worldgen) = common::load();
    std::thread::scope(|scope| {
        for seed in SEEDS {
            let worldgen = worldgen.clone();
            scope.spawn(move || {
                let generator = Generator::new(strata_worldgen::Seed(seed), worldgen.clone());
                let grid = biome_grid(&generator);
                let biomes = worldgen.biomes();
                let contacts = forbidden_contacts(biomes, &grid);
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
                let again = Generator::new(strata_worldgen::Seed(seed), worldgen.clone());
                assert!(
                    grid_in_reverse(&again).eq(grid.iter().step_by(97).rev().copied()),
                    "seed {seed}: biomes changed"
                );
            });
        }
    });
}
