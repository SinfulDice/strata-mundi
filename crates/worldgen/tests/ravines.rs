#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Ravines are rare, and their walls are bare rock: no grass or soil over the void.

mod common;

use common::caves::{Pair, TOP, blocks_of, column_of};
use std::collections::BTreeSet;
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;

const SEEDS: [u64; 4] = [42, 7, 99, 1234];
const SIDES: [(i32, i32, i32); 5] = [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)];

/// Share of a 4096-block square over ravines, sampled every 8 blocks.
fn ravine_share(seed: u64) -> f64 {
    let generator = common::generator(seed);
    let (step, half) = (8, 2048);
    let points: Vec<_> = (-half..half)
        .step_by(step)
        .flat_map(|z| (-half..half).step_by(step).map(move |x| (x, z)))
        .collect();
    let inside = points
        .iter()
        .filter(|&&(x, z)| generator.in_ravine(x, z))
        .count();
    inside as f64 / points.len() as f64
}

#[test]
fn ravines_are_rare_but_present() {
    for seed in SEEDS {
        let share = ravine_share(seed);
        assert!(
            (0.0005..0.02).contains(&share),
            "seed {seed}: {:.3} % of the land over a ravine",
            share * 100.0
        );
    }
}

/// Chunks crossed by a ravine, found on a coarse grid.
fn ravine_chunks(pair: &Pair, count: usize) -> Vec<ColumnPos> {
    let mut found = Vec::new();
    for i in 0..40_000 {
        let (x, z) = ((i % 200) * 16 - 1600, (i / 200) * 16 - 1600);
        if pair.caves.in_ravine(x, z) {
            let pos = column_of((x, z));
            if !found.contains(&pos) {
                found.push(pos);
            }
            if found.len() >= count {
                break;
            }
        }
    }
    found
}

#[test]
fn ravine_walls_are_bare_rock() {
    let mut open_walls = 0;
    for seed in SEEDS {
        let mut pair = Pair::new(seed);
        let worldgen = pair.caves.content().clone();
        let stone = worldgen.terrain.stone;
        let soil: BTreeSet<BlockId> = worldgen
            .biomes()
            .iter()
            .flat_map(|biome| [biome.surface, biome.subsurface])
            .filter(|&block| block != stone)
            .collect();
        for pos in ravine_chunks(&pair, 6) {
            for (x, z) in blocks_of(pos) {
                if !pair.caves.in_ravine(x, z) {
                    continue;
                }
                let surface = pair.plain.surface_height(x, z);
                for y in 1..TOP {
                    if pair.carved(x, y, z) != Some(BlockId::AIR) {
                        continue;
                    }
                    for (dx, dy, dz) in SIDES {
                        let (wall, _) = pair.blocks(x + dx, y + dy, z + dz);
                        assert!(
                            !soil.contains(&wall),
                            "seed {seed}: soil on a ravine wall at ({}, {}, {})",
                            x + dx,
                            y + dy,
                            z + dz
                        );
                        open_walls += usize::from(y + 3 >= surface && wall != BlockId::AIR);
                    }
                }
            }
        }
    }
    assert!(open_walls > 0, "no ravine open at the surface was checked");
}
