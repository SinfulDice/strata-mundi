//! 3D terrain: a block is solid where `height - y + overhang × noise(x, y, z) > 0`.
//! The noise bends the ground into cliffs, overhangs and arches where the relief allows it.
//! It is sampled on a coarse grid aligned on the world (not on the column), so any block's
//! density can be recomputed exactly from anywhere: trees and the spawn rest on the real
//! ground of neighbouring columns.

use crate::Seed;
use crate::noise::Noise;

const STEP_XZ: i32 = 4;
const STEP_Y: i32 = 8;
/// Squashes the noise vertically so the ground bends into ledges rather than towers.
const VERTICAL_STRETCH: f64 = 1.6;

pub struct Overhangs {
    noise: Noise,
}

impl Overhangs {
    /// `scale`: size of the noise's shapes, in blocks.
    pub fn new(seed: Seed, scale: f64) -> Self {
        Self {
            noise: Noise::new(seed, 30, 1.0 / scale, 3),
        }
    }

    fn corner(&self, gx: i32, gy: i32, gz: i32) -> f64 {
        self.noise.at_3d(
            f64::from(gx * STEP_XZ),
            f64::from(gy * STEP_Y) * VERTICAL_STRETCH,
            f64::from(gz * STEP_XZ),
        )
    }

    /// Noise grid covering one column (16×16 blocks) between `low` and `high`.
    pub fn grid(&self, origin: (i32, i32), low: i32, high: i32) -> Grid {
        let first_layer = low.div_euclid(STEP_Y);
        let layers = (high.div_euclid(STEP_Y) - first_layer + 2).max(1) as usize;
        let (gx0, gz0) = (origin.0.div_euclid(STEP_XZ), origin.1.div_euclid(STEP_XZ));
        let values = (0..layers)
            .map(|layer| {
                let gy = first_layer + layer as i32;
                std::array::from_fn(|gz| {
                    std::array::from_fn(|gx| self.corner(gx0 + gx as i32, gy, gz0 + gz as i32))
                })
            })
            .collect();
        Grid {
            first_layer,
            values,
        }
    }

    /// Height of the highest solid block at `(x, z)`, given the plain `height` and the
    /// `overhang` amplitude there.
    pub fn surface(&self, x: i32, z: i32, height: i32, overhang: f64) -> i32 {
        if overhang <= 0.0 {
            return height;
        }
        let reach = overhang.ceil() as i32;
        let (gx, gz) = (x.div_euclid(STEP_XZ), z.div_euclid(STEP_XZ));
        let (fx, fz) = (fraction(x, STEP_XZ), fraction(z, STEP_XZ));
        let layer = |gy: i32| {
            let near = lerp(self.corner(gx, gy, gz), self.corner(gx + 1, gy, gz), fx);
            let far = lerp(
                self.corner(gx, gy, gz + 1),
                self.corner(gx + 1, gy, gz + 1),
                fx,
            );
            lerp(near, far, fz)
        };
        // Scanning downward, two consecutive layers are enough at any time.
        let mut current = i32::MAX;
        let (mut below, mut above) = (0.0, 0.0);
        for y in (height - reach..=height + reach).rev() {
            let gy = y.div_euclid(STEP_Y);
            if gy != current {
                above = if gy + 1 == current {
                    below
                } else {
                    layer(gy + 1)
                };
                below = layer(gy);
                current = gy;
            }
            let noise = lerp(below, above, fraction(y, STEP_Y));
            if is_solid(height, overhang, y, noise) {
                return y;
            }
        }
        height - reach
    }
}

/// Coarse noise values around one column, interpolated per block.
pub struct Grid {
    first_layer: i32,
    values: Vec<[[f64; 5]; 5]>,
}

impl Grid {
    /// `(lx, lz)` local to the column.
    pub fn at(&self, lx: usize, y: i32, lz: usize) -> f64 {
        let step = STEP_XZ as usize;
        let (gx, fx) = (lx / step, (lx % step) as f64 / f64::from(STEP_XZ));
        let (gz, fz) = (lz / step, (lz % step) as f64 / f64::from(STEP_XZ));
        let last = self.values.len() - 1;
        let layer = ((y.div_euclid(STEP_Y) - self.first_layer).max(0) as usize).min(last);
        let plane = |l: &[[f64; 5]; 5]| {
            let near = lerp(l[gz][gx], l[gz][gx + 1], fx);
            let far = lerp(l[gz + 1][gx], l[gz + 1][gx + 1], fx);
            lerp(near, far, fz)
        };
        let below = plane(&self.values[layer]);
        let above = plane(&self.values[(layer + 1).min(last)]);
        lerp(below, above, fraction(y, STEP_Y))
    }
}

pub fn is_solid(height: i32, overhang: f64, y: i32, noise: f64) -> bool {
    f64::from(height - y) + overhang * noise > 0.0
}

fn fraction(value: i32, step: i32) -> f64 {
    f64::from(value.rem_euclid(step)) / f64::from(step)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_and_probe_agree() {
        let overhangs = Overhangs::new(Seed(7), 44.0);
        let grid = overhangs.grid((32, -48), 100, 180);
        let (height, amplitude) = (140, 30.0);
        for (lx, lz) in [(0, 0), (5, 9), (15, 15), (3, 12)] {
            let surface = overhangs.surface(32 + lx as i32, -48 + lz as i32, height, amplitude);
            let solid = |y| is_solid(height, amplitude, y, grid.at(lx, y, lz));
            assert!(solid(surface), "({lx}, {lz}) at {surface}");
            assert!((surface + 1..=height + 30).all(|y| !solid(y)));
        }
    }
}
