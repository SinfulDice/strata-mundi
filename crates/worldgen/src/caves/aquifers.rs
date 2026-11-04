//! Underground water tables: in some regions, caves below a local level are flooded.
//! Each region has one flat level, so cave water never slopes. Regions are square cells
//! of a warped space, so their borders wind. Along a border between two levels, a stone
//! wall spans the heights where one side would be water and the other air.

use crate::Seed;
use crate::content::Caves as Settings;
use crate::noise::Noise;

/// Half-thickness of the walls, in warped blocks.
const WALL: f64 = 2.0;
/// How far borders wander, as a share of the cell size.
const WARP: f64 = 0.3;

pub struct Aquifers {
    seed: Seed,
    warp: (Noise, Noise),
    cell: i32,
    share: f64,
    highest: i32,
    depths: u64,
}

/// Water of one block column.
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// Carved space up to this height fills with water.
    pub level: Option<i32>,
    /// Nothing is carved between these heights (border with a different level).
    wall: Option<(i32, i32)>,
}

impl Table {
    pub fn walls(&self, y: i32) -> bool {
        self.wall
            .is_some_and(|(low, high)| (low..=high).contains(&y))
    }
}

impl Aquifers {
    pub fn new(seed: Seed, settings: &Settings) -> Self {
        let frequency = 1.0 / f64::from(settings.aquifer_cell);
        Self {
            seed,
            warp: (
                Noise::new(seed, 30, frequency, 1),
                Noise::new(seed, 31, frequency, 1),
            ),
            cell: settings.aquifer_cell,
            share: settings.aquifers,
            highest: settings.aquifer_top,
            depths: settings.aquifer_depth as u64 + 1,
        }
    }

    pub fn table(&self, x: i32, z: i32) -> Table {
        let (fx, fz) = (f64::from(x), f64::from(z));
        let reach = f64::from(self.cell) * WARP;
        let wx = fx + reach * self.warp.0.at(fx, fz);
        let wz = fz + reach * self.warp.1.at(fx, fz);
        let size = f64::from(self.cell);
        let cell = ((wx / size).floor() as i32, (wz / size).floor() as i32);
        let level = self.level(cell);
        let side = |w: f64| {
            let local = w.rem_euclid(size);
            if local < WALL {
                -1
            } else if local >= size - WALL {
                1
            } else {
                0
            }
        };
        let (dx, dz) = (side(wx), side(wz));
        let wall = [(dx, 0), (0, dz), (dx, dz)]
            .into_iter()
            .filter(|&offset| offset != (0, 0))
            .filter_map(|(ox, oz)| band(level, self.level((cell.0 + ox, cell.1 + oz))))
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)));
        Table { level, wall }
    }

    fn level(&self, cell: (i32, i32)) -> Option<i32> {
        let roll = hash(self.seed, cell);
        let present = (roll & 0xffff) as f64 / 65536.0 < self.share;
        let depth = ((roll >> 16) % self.depths) as i32;
        present.then_some(self.highest - depth)
    }
}

/// Heights where one side holds water and the other would be open air.
fn band(a: Option<i32>, b: Option<i32>) -> Option<(i32, i32)> {
    match (a, b) {
        (Some(a), Some(b)) if a != b => Some((a.min(b) + 1, a.max(b))),
        (Some(level), None) | (None, Some(level)) => Some((i32::MIN, level)),
        _ => None,
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
