//! Simple tunnels ("spaghetti caves"): carved where two 3D noises are both near zero.
//! Noise is sampled on a coarse grid and interpolated, which is ~60× cheaper.

use crate::Seed;
use crate::noise::Noise;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::SECTION_SIZE;

const STEP: i32 = 4;
const WIDTH: f64 = 0.075;
const LOWEST: i32 = 5;

pub struct Caves {
    a: Noise,
    b: Noise,
}

impl Caves {
    pub fn new(seed: Seed) -> Self {
        Self {
            a: Noise::new(seed, 20, 1.0 / 56.0, 2),
            b: Noise::new(seed, 21, 1.0 / 56.0, 2),
        }
    }

    /// `roofs`: highest y tunnels may reach under each block of the column.
    pub fn carve(&self, column: &mut Column, roofs: &[[i32; 16]; 16], bedrock: BlockId) {
        let top = roofs.iter().flatten().copied().max().unwrap_or(0);
        let layers = (top / STEP + 2) as usize;
        let origin = (column.pos.x * SECTION_SIZE, column.pos.z * SECTION_SIZE);
        let grid = |noise: &Noise| -> Vec<[[f64; 5]; 5]> {
            (0..layers)
                .map(|layer| {
                    let y = f64::from(layer as i32 * STEP) * 1.6;
                    std::array::from_fn(|gz| {
                        std::array::from_fn(|gx| {
                            let x = f64::from(origin.0 + gx as i32 * STEP);
                            let z = f64::from(origin.1 + gz as i32 * STEP);
                            noise.at_3d(x, y, z)
                        })
                    })
                })
                .collect()
        };
        let (a, b) = (grid(&self.a), grid(&self.b));
        for (lz, row) in roofs.iter().enumerate() {
            for (lx, &roof) in row.iter().enumerate() {
                for y in LOWEST..=roof {
                    let near_zero = |g: &[[[f64; 5]; 5]]| interpolate(g, lx, y, lz).abs() < WIDTH;
                    let carvable = column.get(lx as u32, y, lz as u32) != bedrock;
                    if carvable && near_zero(&a) && near_zero(&b) {
                        column.set(lx as u32, y, lz as u32, BlockId::AIR);
                    }
                }
            }
        }
    }
}

/// Trilinear interpolation inside the coarse grid.
fn interpolate(grid: &[[[f64; 5]; 5]], x: usize, y: i32, z: usize) -> f64 {
    let step = STEP as usize;
    let (gx, fx) = (x / step, (x % step) as f64 / f64::from(STEP));
    let (gz, fz) = (z / step, (z % step) as f64 / f64::from(STEP));
    let gy = (y / STEP) as usize;
    let fy = f64::from(y % STEP) / f64::from(STEP);
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let plane = |layer: &[[f64; 5]; 5]| {
        let near = lerp(layer[gz][gx], layer[gz][gx + 1], fx);
        let far = lerp(layer[gz + 1][gx], layer[gz + 1][gx + 1], fx);
        lerp(near, far, fz)
    };
    let below = plane(&grid[gy.min(grid.len() - 1)]);
    let above = plane(&grid[(gy + 1).min(grid.len() - 1)]);
    lerp(below, above, fy)
}
