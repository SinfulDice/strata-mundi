#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::caves::{Pair, TOP, blocks_of, column_of};
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;
use strata_worldgen::Generator;

const SEEDS: [u64; 3] = [42, 7, 99];

const SIDES: [(i32, i32, i32); 6] = [
    (1, 0, 0),
    (-1, 0, 0),
    (0, 0, 1),
    (0, 0, -1),
    (0, 1, 0),
    (0, -1, 0),
];

/// Columns on the shore, where land meets the sea.
fn coast_columns(generator: &Generator, count: usize) -> Vec<ColumnPos> {
    let sea_level = generator.content().terrain.sea_level;
    (0..6400)
        .map(|i| ((i % 80) * 40 - 1600, (i / 80) * 40 - 1600))
        .filter(|&(x, z)| {
            let (sample, biome) = generator.sample(x, z);
            !biome.river && (sea_level - 3..=sea_level + 2).contains(&sample.height)
        })
        .take(count)
        .map(column_of)
        .collect()
}

/// No carved block touches the sea or a river.
#[test]
fn caves_never_open_seas_or_rivers() {
    for seed in SEEDS {
        let mut pair = Pair::new(seed);
        let terrain = pair.caves.content().terrain.clone();
        let mut columns = coast_columns(&pair.caves, 4);
        columns.extend(
            common::river_points(&pair.caves)
                .into_iter()
                .take(4)
                .map(column_of),
        );
        assert!(columns.len() >= 6, "seed {seed}: few shore columns");
        let mut carved = 0;
        for pos in columns {
            for (x, y, z) in carved_air(&mut pair, pos, terrain.sea_level + 2) {
                for (dx, dy, dz) in SIDES {
                    let (_, without) = pair.blocks(x + dx, y + dy, z + dz);
                    let sea = without == terrain.water || without == terrain.ice;
                    assert!(!sea, "seed {seed}: ({x}, {y}, {z}) opens the sea");
                }
                carved += 1;
            }
        }
        assert!(carved > 0, "seed {seed}: no cave under the shore");
    }
}

fn carved_air(pair: &mut Pair, pos: ColumnPos, top: i32) -> Vec<(i32, i32, i32)> {
    let mut found = Vec::new();
    for (x, z) in blocks_of(pos) {
        for y in 1..=top {
            if pair.carved(x, y, z) == Some(BlockId::AIR) {
                found.push((x, y, z));
            }
        }
    }
    found
}

/// Where aquifers of different levels meet, cave water never has carved air beside or
/// under it, neighbouring columns included.
#[test]
fn aquifer_water_never_hangs() {
    for seed in SEEDS {
        let mut pair = Pair::new(seed);
        let borders: Vec<_> = common::caves::scan()
            .filter(|&pos| pair.crosses_aquifer_border(pos))
            .take(3)
            .collect();
        assert!(!borders.is_empty(), "seed {seed}: no aquifer border");
        for pos in borders {
            for (x, z) in blocks_of(pos) {
                for y in 1..TOP {
                    if !pair.is_cave_water(x, y, z) {
                        continue;
                    }
                    for (dx, dy, dz) in SIDES.into_iter().filter(|side| side.1 <= 0) {
                        let open = pair.carved(x + dx, y + dy, z + dz) == Some(BlockId::AIR);
                        assert!(!open, "seed {seed}: water at ({x}, {y}, {z}) hangs");
                    }
                }
            }
        }
    }
}

#[derive(Default, Debug)]
struct Found {
    cavern_layers: usize,
    ravine_columns: usize,
    aquifer_water: usize,
    lava: usize,
}

/// Each reference seed has large caverns, ravines, flooded caves and lava lakes, and
/// lava stays deep.
#[test]
fn reference_seeds_have_every_kind_of_cave() {
    for seed in SEEDS {
        let mut pair = Pair::new(seed);
        let caves = pair.caves.content().terrain.caves.clone().unwrap();
        let mut found = Found::default();
        for pos in (-3..3).flat_map(|cx| {
            (-3..3).map(move |cz| ColumnPos {
                x: cx * 5,
                z: cz * 5,
            })
        }) {
            for y in 1..TOP {
                let mut layer = 0;
                for (x, z) in blocks_of(pos) {
                    match pair.carved(x, y, z) {
                        Some(BlockId::AIR) => layer += 1,
                        Some(block) if block == caves.lava => {
                            assert!(y <= caves.lava_level, "seed {seed}: lava at {y}");
                            found.lava += 1;
                        }
                        Some(block) if block == caves.water => found.aquifer_water += 1,
                        _ => {}
                    }
                }
                found.cavern_layers += usize::from(layer >= 200);
            }
            found.ravine_columns += usize::from(pair.tallest_air_run(pos) >= 60);
        }
        assert!(found.cavern_layers > 0, "seed {seed}: {found:?}");
        assert!(found.ravine_columns > 0, "seed {seed}: {found:?}");
        assert!(found.aquifer_water > 0, "seed {seed}: {found:?}");
        assert!(found.lava > 0, "seed {seed}: {found:?}");
    }
}

/// The player starts on solid ground, with no cave just below.
#[test]
fn spawn_stands_over_solid_ground() {
    for seed in SEEDS {
        let mut pair = Pair::new(seed);
        let spawn = pair.caves.spawn_point();
        let (below, _) = pair.blocks(spawn.x, spawn.y - 1, spawn.z);
        let terrain = pair.caves.content().terrain.clone();
        assert!(
            ![BlockId::AIR, terrain.water].contains(&below),
            "seed {seed}"
        );
        for depth in 1..=3 {
            let carved = pair.carved(spawn.x, spawn.y - depth, spawn.z);
            assert_eq!(carved, None, "seed {seed}: cave {depth} blocks under spawn");
        }
    }
}
