#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::HashMap;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;
use strata_worldgen::Generator;

const SEEDS: [u64; 3] = [42, 7, 99];
const TOP: i32 = 320;

/// The same world with and without caves, columns generated on demand.
struct Pair {
    caves: Generator,
    plain: Generator,
    columns: HashMap<ColumnPos, (Column, Column)>,
}

impl Pair {
    fn new(seed: u64) -> Self {
        Self {
            caves: common::generator(seed),
            plain: common::generator_without_caves(seed),
            columns: HashMap::new(),
        }
    }

    fn column(&mut self, pos: ColumnPos) -> &(Column, Column) {
        let (caves, plain) = (&self.caves, &self.plain);
        self.columns
            .entry(pos)
            .or_insert_with(|| (caves.generate(pos), plain.generate(pos)))
    }

    /// `(with caves, without caves)` at a world position.
    fn blocks(&mut self, x: i32, y: i32, z: i32) -> (BlockId, BlockId) {
        let pos = ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        };
        let (lx, lz) = (x.rem_euclid(16) as u32, z.rem_euclid(16) as u32);
        let (caves, plain) = self.column(pos);
        (caves.get(lx, y, lz), plain.get(lx, y, lz))
    }
}

fn column_of((x, z): (i32, i32)) -> ColumnPos {
    ColumnPos {
        x: x.div_euclid(16),
        z: z.div_euclid(16),
    }
}

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

const SIDES: [(i32, i32, i32); 6] = [
    (1, 0, 0),
    (-1, 0, 0),
    (0, 0, 1),
    (0, 0, -1),
    (0, 1, 0),
    (0, -1, 0),
];

/// No carved block touches the sea or a river, and cave water never hangs beside or
/// over carved air.
#[test]
fn caves_never_open_seas_or_rivers() {
    for seed in SEEDS {
        let mut pair = Pair::new(seed);
        let terrain = pair.caves.content().terrain.clone();
        let (water, sea_level) = (terrain.water, terrain.sea_level);
        let mut columns = coast_columns(&pair.caves, 4);
        columns.extend(
            common::river_points(&pair.caves)
                .into_iter()
                .take(4)
                .map(column_of),
        );
        assert!(columns.len() >= 6, "seed {seed}: few shore columns");
        let mut carved_near_water = 0;
        for pos in columns {
            for (x, y, z) in carved_air(&mut pair, pos, sea_level + 2) {
                for (dx, dy, dz) in SIDES {
                    let (with, without) = pair.blocks(x + dx, y + dy, z + dz);
                    let sea = without == water || without == terrain.ice;
                    let hanging = with == water && dy >= 0;
                    assert!(!sea && !hanging, "seed {seed}: ({x}, {y}, {z}) opens water");
                }
                carved_near_water += 1;
            }
        }
        assert!(
            carved_near_water > 0,
            "seed {seed}: no cave under the shore"
        );
    }
}

/// Air carved by caves in a column, up to `top`.
fn carved_air(pair: &mut Pair, pos: ColumnPos, top: i32) -> Vec<(i32, i32, i32)> {
    let (caves, plain) = pair.column(pos);
    let mut found = Vec::new();
    for y in 1..=top {
        for lz in 0..16 {
            for lx in 0..16 {
                let open = caves.get(lx, y, lz) == BlockId::AIR;
                if open && plain.get(lx, y, lz) != BlockId::AIR {
                    found.push((pos.x * 16 + lx as i32, y, pos.z * 16 + lz as i32));
                }
            }
        }
    }
    found
}

#[derive(Default, Debug)]
struct Found {
    cavern_layers: usize,
    ravine_columns: usize,
    aquifer_water: usize,
    lava: usize,
}

/// Each reference seed has large caverns, ravines, flooded caves and lava lakes; lava
/// stays deep and each column's cave water has a single flat level.
#[test]
fn reference_seeds_have_every_kind_of_cave() {
    for seed in SEEDS {
        let mut pair = Pair::new(seed);
        let terrain = pair.caves.content().terrain.clone();
        let ground_only = common::feature_blocks(pair.caves.content());
        let mut found = Found::default();
        for (cx, cz) in (-3..3).flat_map(|cx| (-3..3).map(move |cz| (cx * 5, cz * 5))) {
            let (caves, plain) = pair.column(ColumnPos { x: cx, z: cz }).clone();
            let carved = |x: u32, y: i32, z: u32| {
                let before = plain.get(x, y, z);
                before != BlockId::AIR
                    && !ground_only.contains(&before)
                    && caves.get(x, y, z) != before
            };
            let mut levels = Vec::new();
            for y in 1..TOP {
                let mut layer = 0;
                for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
                    if !carved(x, y, z) {
                        continue;
                    }
                    let block = caves.get(x, y, z);
                    layer += usize::from(block == BlockId::AIR);
                    if block == terrain.caves.lava {
                        assert!(y <= terrain.caves.lava_level, "seed {seed}: lava at {y}");
                        found.lava += 1;
                    }
                    if block == terrain.water {
                        found.aquifer_water += 1;
                        if caves.get(x, y + 1, z) == BlockId::AIR {
                            levels.push(y);
                        }
                    }
                }
                found.cavern_layers += usize::from(layer >= 200);
            }
            levels.dedup();
            assert!(levels.len() <= 1, "seed {seed}: cave water at {levels:?}");
            for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
                let deepest_run = (1..TOP)
                    .scan(0, |run, y| {
                        *run = if carved(x, y, z) { *run + 1 } else { 0 };
                        Some(*run)
                    })
                    .max()
                    .unwrap_or(0);
                found.ravine_columns += usize::from(deepest_run >= 60);
            }
        }
        assert!(found.cavern_layers > 0, "seed {seed}: {found:?}");
        assert!(found.ravine_columns > 0, "seed {seed}: {found:?}");
        assert!(found.aquifer_water > 0, "seed {seed}: {found:?}");
        assert!(found.lava > 0, "seed {seed}: {found:?}");
    }
}
