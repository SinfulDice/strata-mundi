//! Cave noise sampled every 4 blocks on a grid aligned on the world, then interpolated.

const STEP: i32 = 4;
const CORNERS: usize = 16 / STEP as usize + 1;

type Layer = [[f64; CORNERS]; CORNERS];

pub struct CaveGrid {
    layers: Vec<Layer>,
}

impl CaveGrid {
    /// `value(x, y, z)` at each corner of a column from y = 0 up to `top`.
    pub fn sample(origin: (i32, i32), top: i32, value: impl Fn(i32, i32, i32) -> f64) -> Self {
        let count = (top.max(0) / STEP + 2) as usize;
        let layers = (0..count)
            .map(|layer| {
                let y = layer as i32 * STEP;
                std::array::from_fn(|gz| {
                    std::array::from_fn(|gx| {
                        value(origin.0 + gx as i32 * STEP, y, origin.1 + gz as i32 * STEP)
                    })
                })
            })
            .collect();
        Self { layers }
    }

    /// `(lx, lz)` local to the column.
    pub fn at(&self, lx: usize, y: i32, lz: usize) -> f64 {
        let last = self.layers.len() - 1;
        let (gx, gy, gz) = (
            lx / STEP as usize,
            (y.max(0) / STEP) as usize,
            lz / STEP as usize,
        );
        trilinear(fractions(lx as i32, y, lz as i32), |dx, dy, dz| {
            self.layers[(gy + dy).min(last)][gz + dz][gx + dx]
        })
    }
}

/// The value a grid would interpolate at one block, without sampling a whole column.
pub fn at_point(x: i32, y: i32, z: i32, value: impl Fn(i32, i32, i32) -> f64) -> f64 {
    let y = y.max(0);
    let corner = |v: i32| v.div_euclid(STEP) * STEP;
    let base = (corner(x), corner(y), corner(z));
    trilinear(fractions(x, y, z), |dx, dy, dz| {
        let offset = |d: usize| d as i32 * STEP;
        value(
            base.0 + offset(dx),
            base.1 + offset(dy),
            base.2 + offset(dz),
        )
    })
}

fn fractions(x: i32, y: i32, z: i32) -> (f64, f64, f64) {
    let part = |v: i32| f64::from(v.rem_euclid(STEP)) / f64::from(STEP);
    (part(x), part(y.max(0)), part(z))
}

fn trilinear((fx, fy, fz): (f64, f64, f64), corner: impl Fn(usize, usize, usize) -> f64) -> f64 {
    let plane = |dy: usize| {
        let near = lerp(corner(0, dy, 0), corner(1, dy, 0), fx);
        let far = lerp(corner(0, dy, 1), corner(1, dy, 1), fx);
        lerp(near, far, fz)
    };
    lerp(plane(0), plane(1), fy)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_matches_the_column_grid() {
        let value = |x: i32, y: i32, z: i32| f64::from(x * 7 - y * y + z * 3).sin();
        let grid = CaveGrid::sample((-32, 48), 40, value);
        for (lx, y, lz) in [(0, 0, 0), (3, 17, 9), (15, 40, 15), (6, 33, 1)] {
            let point = at_point(lx as i32 - 32, y, lz as i32 + 48, value);
            assert!((grid.at(lx, y, lz) - point).abs() < 1e-12);
        }
    }
}
