#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;

const SEEDS: [u64; 3] = [42, 7, 99];

#[test]
fn reference_seeds_have_overhangs() {
    for seed in SEEDS {
        let generator = common::generator_without_caves(seed);
        let grown = common::feature_blocks(generator.content());
        let sea_level = generator.content().terrain.sea_level;
        let overhangs: usize = common::rugged_columns(&generator, 12)
            .into_iter()
            .map(|pos| {
                let column = generator.generate(pos);
                let mut count = 0;
                for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
                    let ground = |y| {
                        let block = column.get(x, y, z);
                        block != BlockId::AIR && !grown.contains(&block)
                    };
                    let top = (sea_level..420).rev().find(|&y| ground(y)).unwrap_or(0);
                    count += usize::from((sea_level + 2..top).any(|y| !ground(y) && ground(y - 1)));
                }
                count
            })
            .sum();
        assert!(overhangs > 0, "seed {seed}: no overhang");
    }
}

#[test]
fn reference_seeds_have_cliffs() {
    for seed in SEEDS {
        let generator = common::generator(seed);
        let cliff = common::rugged_columns(&generator, 40)
            .into_iter()
            .any(|pos| {
                let (x0, z0) = (pos.x * 16, pos.z * 16);
                (0..15).any(|i| {
                    let a = generator.surface_height(x0 + i, z0 + 8);
                    let b = generator.surface_height(x0 + i + 1, z0 + 8);
                    (a - b).abs() >= 8
                })
            });
        assert!(cliff, "seed {seed}: no cliff");
    }
}

#[test]
fn rivers_hold_water_inland() {
    let generator = common::generator(42);
    let sea_level = generator.content().terrain.sea_level;
    let water = generator.content().terrain.water;
    let points = common::river_points(&generator);
    assert!(points.len() > 20, "only {} river points", points.len());
    for &(x, z) in points.iter().take(5) {
        let column = generator.generate(ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        });
        let (lx, lz) = (x.rem_euclid(16) as u32, z.rem_euclid(16) as u32);
        assert_eq!(column.get(lx, sea_level, lz), water, "({x}, {z})");
    }
}

/// Trees and arches rest on the real ground, also across column borders and where
/// columns with and without overhangs meet.
#[test]
fn surface_height_matches_generated_columns() {
    for seed in SEEDS {
        let generator = common::generator_without_caves(seed);
        let grown = common::feature_blocks(generator.content());
        for pos in common::rugged_columns(&generator, 6) {
            let column = generator.generate(pos);
            for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
                let expected =
                    generator.surface_height(pos.x * 16 + x as i32, pos.z * 16 + z as i32);
                let ground = |y| {
                    let block = column.get(x, y, z);
                    block != BlockId::AIR && !grown.contains(&block)
                };
                let top = (0..512).rev().find(|&y| ground(y)).unwrap_or(0);
                assert_eq!(top, expected, "seed {seed} {pos:?} ({x}, {z})");
            }
        }
    }
}
