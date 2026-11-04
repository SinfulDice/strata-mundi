//! Giant ore veins, rare and rich, hinted at by stones on the ground above them. The world
//! is cut into square cells; a hash of each cell decides whether it holds a vein, where and
//! how it lies. A column looks at every cell its blocks may reach, so the result never
//! depends on the order in which columns are generated.

mod def;

pub use def::Deposit;

use crate::Seed;
use crate::content::{Feature, Terrain, WorldgenContent};
use crate::noise::Noise;
use crate::rng::Rng;
use std::f64::consts::TAU;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE, WORLD_HEIGHT};

/// How much the noise swells or thins the vein, as a share of its radius.
const WOBBLE: f64 = 0.35;
const CELL_SALT: u64 = 7000;
const ORE_SALT: u64 = 7100;
const SIGN_SALT: u64 = 7200;
const WOBBLE_SALT: u64 = 7300;

/// One vein placed in the world: a lens around a straight segment, thickest at the middle.
#[derive(Debug, Clone, Copy)]
pub struct Vein {
    /// Index into `WorldgenContent::features`.
    pub feature: usize,
    pub center: BlockPos,
    start: [f64; 3],
    axis: [f64; 3],
    length: f64,
    radius: f64,
}

impl Vein {
    fn in_cell(seed: Seed, feature: usize, deposit: &Deposit, cell: ColumnPos) -> Option<Self> {
        let salt = deposit.salt.wrapping_add(CELL_SALT);
        let mut rng = Rng::for_column(seed, cell, salt);
        if rng.unit() >= deposit.chance {
            return None;
        }
        let side = deposit.cell;
        let center = BlockPos {
            x: cell.x * side + rng.range(0, side - 1),
            y: rng.range(deposit.min_y, deposit.max_y),
            z: cell.z * side + rng.range(0, side - 1),
        };
        let length = f64::from(rng.range(deposit.length.0 as i32, deposit.length.1 as i32));
        let thickness = rng.range(deposit.thickness.0 as i32, deposit.thickness.1 as i32);
        let yaw = rng.unit() * TAU;
        let pitch = (rng.unit() - 0.5) * 0.5;
        let axis = [
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        ];
        let middle = [center.x, center.y, center.z].map(f64::from);
        Some(Self {
            feature,
            center,
            start: std::array::from_fn(|i| middle[i] - axis[i] * length / 2.0),
            axis,
            length,
            radius: f64::from(thickness) / 2.0,
        })
    }

    /// `(t, distance)`: where along the vein (0 to 1) the nearest point lies, and how far.
    /// `flat` ignores heights, for the footprint on the ground.
    fn nearest(&self, p: [f64; 3], flat: bool) -> (f64, f64) {
        let (t, d, axis) = self.project(p, flat);
        let t = t.clamp(0.0, 1.0);
        let off: [f64; 3] = std::array::from_fn(|i| d[i] - axis[i] * self.length * t);
        (t, dot(off, off).sqrt())
    }

    /// Unclamped position of `p` along the vein, with the offset and axis used.
    fn project(&self, p: [f64; 3], flat: bool) -> (f64, [f64; 3], [f64; 3]) {
        let keep = if flat { [1.0, 0.0, 1.0] } else { [1.0; 3] };
        let d: [f64; 3] = std::array::from_fn(|i| (p[i] - self.start[i]) * keep[i]);
        let axis: [f64; 3] = std::array::from_fn(|i| self.axis[i] * keep[i]);
        let span = self.length * dot(axis, axis);
        let t = if span > 0.0 { dot(d, axis) / span } else { 0.5 };
        (t, d, axis)
    }

    /// Radius at `t`: tapers towards both ends.
    fn lens(&self, t: f64) -> f64 {
        let from_middle = 2.0 * t - 1.0;
        self.radius * (0.3 + 0.7 * (1.0 - from_middle * from_middle).sqrt())
    }

    fn widest(&self) -> f64 {
        self.radius * (1.0 + WOBBLE)
    }

    /// Heights the vein may reach above `(x, z)`: only the part of the segment whose
    /// footprint lies within `widest` of the point can hold blocks there.
    fn heights_at(&self, x: f64, z: f64) -> (i32, i32) {
        let widest = self.widest();
        let (t, _, axis) = self.project([x, 0.0, z], true);
        let flat_length = self.length * dot(axis, axis).sqrt();
        let spread = if flat_length > 0.0 {
            widest / flat_length
        } else {
            1.0
        };
        let y_at = |t: f64| self.start[1] + self.axis[1] * self.length * t.clamp(0.0, 1.0);
        let (a, b) = (y_at(t - spread), y_at(t + spread));
        let low = (a.min(b) - widest).floor() as i32;
        let high = (a.max(b) + widest).ceil() as i32;
        (low.max(1), high.min(WORLD_HEIGHT - 1))
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub struct Deposits {
    wobble: Noise,
}

impl Deposits {
    pub fn new(seed: Seed) -> Self {
        Self {
            wobble: Noise::new(seed, WOBBLE_SALT, 1.0 / 12.0, 2),
        }
    }

    /// Veins with blocks possibly inside `min..=max` (horizontal block coordinates).
    pub fn veins(
        seed: Seed,
        content: &WorldgenContent,
        min: (i32, i32),
        max: (i32, i32),
    ) -> impl Iterator<Item = Vein> + '_ {
        content.terrain.deposits.iter().flat_map(move |&feature| {
            let deposit = match content.features.get(feature) {
                Some(Feature::Deposit(deposit)) => Some(deposit),
                _ => None,
            };
            deposit.into_iter().flat_map(move |deposit| {
                let (reach, side) = (deposit.reach(), deposit.cell);
                let cells = move |low: i32, high: i32| {
                    (low - reach).div_euclid(side)..=(high + reach).div_euclid(side)
                };
                let near = move |v: &Vein| {
                    (min.0 - reach..=max.0 + reach).contains(&v.center.x)
                        && (min.1 - reach..=max.1 + reach).contains(&v.center.z)
                };
                cells(min.1, max.1)
                    .flat_map(move |cz| {
                        cells(min.0, max.0).map(move |cx| ColumnPos { x: cx, z: cz })
                    })
                    .filter_map(move |cell| Vein::in_cell(seed, feature, deposit, cell))
                    .filter(near)
            })
        })
    }

    /// Writes the veins reaching `column` (only into stone and deep stone) and their signs
    /// on the ground. `tops` = highest solid block under each `[z][x]`.
    pub fn place(
        &self,
        seed: Seed,
        content: &WorldgenContent,
        column: &mut Column,
        tops: &[[i32; 16]; 16],
    ) {
        let origin = (column.pos.x * SECTION_SIZE, column.pos.z * SECTION_SIZE);
        let max = (origin.0 + SECTION_SIZE - 1, origin.1 + SECTION_SIZE - 1);
        for vein in Self::veins(seed, content, origin, max) {
            let Some(Feature::Deposit(deposit)) = content.features.get(vein.feature) else {
                continue;
            };
            let place = Placing {
                seed,
                deposit,
                vein: &vein,
                origin,
            };
            place.vein(column, &self.wobble, &content.terrain);
            place.signs(column, tops);
        }
    }
}

struct Placing<'a> {
    seed: Seed,
    deposit: &'a Deposit,
    vein: &'a Vein,
    origin: (i32, i32),
}

impl Placing<'_> {
    fn vein(&self, column: &mut Column, wobble: &Noise, terrain: &Terrain) {
        let widest = self.vein.widest();
        let salt = self.deposit.salt.wrapping_add(ORE_SALT);
        for lz in 0..16 {
            for lx in 0..16 {
                let (x, z) = (self.origin.0 + lx, self.origin.1 + lz);
                let (fx, fz) = (f64::from(x), f64::from(z));
                if self.vein.nearest([fx, 0.0, fz], true).1 > widest {
                    continue;
                }
                let (low, high) = self.vein.heights_at(fx, fz);
                for y in low..=high {
                    let deep = match column.get(lx as u32, y, lz as u32) {
                        h if h == terrain.stone => false,
                        h if h == terrain.deep_stone => true,
                        _ => continue,
                    };
                    let p = [fx, f64::from(y), fz];
                    let (t, distance) = self.vein.nearest(p, false);
                    if distance > widest {
                        continue;
                    }
                    let lens = self.vein.lens(t);
                    // The noise only matters near the edge: the core is always inside.
                    let inside = distance <= lens * (1.0 - WOBBLE)
                        || distance <= lens * (1.0 + WOBBLE * wobble.at_3d(p[0], p[1], p[2]));
                    if !inside {
                        continue;
                    }
                    let ore = roll(self.seed, (x, y, z), salt) < self.deposit.ore_share;
                    column.set(lx as u32, y, lz as u32, block_for(self.deposit, ore, deep));
                }
            }
        }
    }

    /// A few ground blocks right above the vein's footprint, never under water.
    fn signs(&self, column: &mut Column, tops: &[[i32; 16]; 16]) {
        let Some(sign) = self.deposit.sign else {
            return;
        };
        let salt = self.deposit.salt.wrapping_add(SIGN_SALT);
        for (lz, row) in tops.iter().enumerate() {
            for (lx, &top) in row.iter().enumerate() {
                let (x, z) = (self.origin.0 + lx as i32, self.origin.1 + lz as i32);
                let (t, distance) = self.vein.nearest([f64::from(x), 0.0, f64::from(z)], true);
                if distance > self.vein.lens(t)
                    || roll(self.seed, (x, 0, z), salt) >= self.deposit.sign_share
                {
                    continue;
                }
                let (cx, cz) = (lx as u32, lz as u32);
                let ground = column.get(cx, top, cz);
                if self.deposit.sign_on.contains(&ground)
                    && column.get(cx, top + 1, cz) == BlockId::AIR
                {
                    column.set(cx, top, cz, sign);
                }
            }
        }
    }
}

fn block_for(deposit: &Deposit, ore: bool, deep: bool) -> BlockId {
    match (ore, deep) {
        (true, false) => deposit.ore,
        (true, true) => deposit.deep_ore,
        (false, false) => deposit.filler,
        (false, true) => deposit.deep_filler,
    }
}

/// Uniform in `[0, 1)`, from a block position.
fn roll(seed: Seed, (x, y, z): (i32, i32, i32), salt: u64) -> f64 {
    let salt = salt.wrapping_add((y as u32 as u64) << 16);
    Rng::for_column(seed, ColumnPos { x, z }, salt).unit()
}
