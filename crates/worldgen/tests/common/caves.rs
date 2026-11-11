//! The same world with and without caves, to tell carved blocks apart.

use std::collections::HashMap;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;
use strata_worldgen::Generator;

pub const TOP: i32 = 320;

pub struct Pair {
    pub caves: Generator,
    pub plain: Generator,
    ground_only: Vec<BlockId>,
    columns: HashMap<ColumnPos, (Column, Column)>,
}

impl Pair {
    pub fn new(seed: u64) -> Self {
        let caves = super::generator(seed);
        Self {
            ground_only: super::feature_blocks(caves.content()),
            caves,
            plain: super::generator_without_caves(seed),
            columns: HashMap::new(),
        }
    }

    pub fn column(&mut self, pos: ColumnPos) -> &(Column, Column) {
        let (caves, plain) = (&self.caves, &self.plain);
        self.columns
            .entry(pos)
            .or_insert_with(|| (caves.generate(pos), plain.generate(pos)))
    }

    /// `(with caves, without caves)` at a world position.
    pub fn blocks(&mut self, x: i32, y: i32, z: i32) -> (BlockId, BlockId) {
        let (lx, lz) = (x.rem_euclid(16) as u32, z.rem_euclid(16) as u32);
        let (caves, plain) = self.column(column_of((x, z)));
        (caves.get(lx, y, lz), plain.get(lx, y, lz))
    }

    /// The block caves left at a world position, if they changed the ground there.
    pub fn carved(&mut self, x: i32, y: i32, z: i32) -> Option<BlockId> {
        let (with, without) = self.blocks(x, y, z);
        let ground = without != BlockId::AIR && !self.ground_only.contains(&without);
        (ground && with != without).then_some(with)
    }

    /// Water filling caves (aquifers), as opposed to seas and rivers.
    pub fn is_cave_water(&mut self, x: i32, y: i32, z: i32) -> bool {
        let water = self.caves.content().terrain.water;
        self.carved(x, y, z) == Some(water)
    }

    /// Longest vertical run of carved air in the column.
    pub fn tallest_air_run(&mut self, pos: ColumnPos) -> i32 {
        let mut tallest = 0;
        for (x, z) in blocks_of(pos) {
            let mut run = 0;
            for y in 1..TOP {
                let air = self.carved(x, y, z) == Some(BlockId::AIR);
                run = if air { run + 1 } else { 0 };
                tallest = tallest.max(run);
            }
        }
        tallest
    }

    /// Whether a ravine crosses the column and carves a tall gap there.
    pub fn has_ravine(&mut self, pos: ColumnPos) -> bool {
        let crossed = blocks_of(pos).any(|(x, z)| self.caves.in_ravine(x, z));
        crossed && self.tallest_air_run(pos) >= 25
    }

    /// Whether the column holds cave water with carved air at or below its level, which
    /// only happens across a border between two aquifer regions.
    pub fn crosses_aquifer_border(&mut self, pos: ColumnPos) -> bool {
        let blocks: Vec<_> = blocks_of(pos).collect();
        let highest_water = (1..TOP)
            .rev()
            .find(|&y| blocks.iter().any(|&(x, z)| self.is_cave_water(x, y, z)));
        let Some(highest) = highest_water else {
            return false;
        };
        (1..=highest).any(|y| {
            blocks
                .iter()
                .any(|&(x, z)| self.carved(x, y, z) == Some(BlockId::AIR))
        })
    }

    pub fn find(&mut self, test: impl Fn(&mut Self, ColumnPos) -> bool) -> Option<ColumnPos> {
        scan().find(|&pos| test(self, pos))
    }
}

/// Columns spread over a 1000-block square around the origin.
pub fn scan() -> impl Iterator<Item = ColumnPos> {
    (0..256).map(|i| ColumnPos {
        x: (i % 16) * 4 - 32,
        z: (i / 16) * 4 - 32,
    })
}

pub fn blocks_of(pos: ColumnPos) -> impl Iterator<Item = (i32, i32)> {
    (0..16).flat_map(move |lz| (0..16).map(move |lx| (pos.x * 16 + lx, pos.z * 16 + lz)))
}

pub fn column_of((x, z): (i32, i32)) -> ColumnPos {
    ColumnPos {
        x: x.div_euclid(16),
        z: z.div_euclid(16),
    }
}
