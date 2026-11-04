//! Underground water tables: in some regions, caves below a local level are flooded.
//! Each region (a square cell) has one flat level, so cave water never slopes; a stone
//! wall stays along borders between regions of different levels so water never hangs.

use crate::Seed;
use crate::content::Caves as Settings;

const CELL: i32 = 64;
const WALL: i32 = 2;
const DEPTHS: u64 = 48;

pub struct Aquifers {
    seed: Seed,
    share: f64,
    highest: i32,
}

/// Water of one block column.
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// Carved space up to this height fills with water.
    pub level: Option<i32>,
    /// Nothing is carved up to this height (border with a different water level).
    pub wall: Option<i32>,
}

impl Aquifers {
    pub fn new(seed: Seed, settings: &Settings) -> Self {
        Self {
            seed,
            share: settings.aquifers,
            highest: settings.aquifer_top,
        }
    }

    pub fn table(&self, x: i32, z: i32) -> Table {
        let cell = (x.div_euclid(CELL), z.div_euclid(CELL));
        let level = self.level(cell);
        let side = |v: i32| match v.rem_euclid(CELL) {
            local if local < WALL => -1,
            local if local >= CELL - WALL => 1,
            _ => 0,
        };
        let (dx, dz) = (side(x), side(z));
        let wall = [(dx, 0), (0, dz), (dx, dz)]
            .into_iter()
            .filter(|&offset| offset != (0, 0))
            .map(|(ox, oz)| self.level((cell.0 + ox, cell.1 + oz)))
            .filter(|&other| other != level)
            .map(|other| other.max(level))
            .max()
            .flatten();
        Table { level, wall }
    }

    fn level(&self, cell: (i32, i32)) -> Option<i32> {
        let roll = hash(self.seed, cell);
        let present = (roll & 0xffff) as f64 / 65536.0 < self.share;
        let depth = ((roll >> 16) % DEPTHS) as i32;
        present.then_some(self.highest - depth)
    }
}

fn hash(seed: Seed, (x, z): (i32, i32)) -> u64 {
    let mut h = seed.0 ^ 0xA0F1_7E55;
    h ^= (x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= (z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^ (h >> 29)
}
