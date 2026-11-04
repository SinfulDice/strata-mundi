//! Giant ore veins, rare and rich, hinted at by stones on the ground above them. The world
//! is cut into square cells; a hash of each cell decides whether it holds a vein, where and
//! how it lies. A column looks at every cell its blocks may reach, so the result never
//! depends on the order in which columns are generated.

use crate::Seed;
use crate::content::{Feature, Resolver, Terrain, WorldgenContent};
use crate::defs::FeatureDef;
use crate::noise::Noise;
use crate::rng::Rng;
use std::f64::consts::TAU;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE, WORLD_HEIGHT};

/// A `Deposit` feature with blocks resolved and sizes bounded.
#[derive(Debug, Clone)]
pub struct Deposit {
    pub ore: BlockId,
    pub deep_ore: BlockId,
    pub filler: BlockId,
    pub deep_filler: BlockId,
    pub ore_share: f64,
    pub length: (u32, u32),
    pub thickness: (u32, u32),
    pub min_y: i32,
    pub max_y: i32,
    pub cell: i32,
    pub chance: f64,
    pub sign: Option<BlockId>,
    pub sign_share: f64,
    pub sign_on: Vec<BlockId>,
}

const MAX_LENGTH: u32 = 128;
const MAX_THICKNESS: u32 = 24;
/// How much the noise swells or thins the vein, as a share of its radius.
const WOBBLE: f64 = 0.35;
const CELL_SALT: u64 = 7000;
const ORE_SALT: u64 = 7100;
const SIGN_SALT: u64 = 7200;
const WOBBLE_SALT: u64 = 7300;

impl Deposit {
    /// `None` if `def` is not a deposit. Bounded so that pack data can't make generation
    /// absurdly slow.
    pub(crate) fn resolve(def: &FeatureDef, r: &mut Resolver<'_>) -> Option<Self> {
        let FeatureDef::Deposit {
            ore,
            deep_ore,
            filler,
            deep_filler,
            ore_share,
            length,
            thickness,
            min_y,
            max_y,
            cell,
            chance,
            sign,
            sign_share,
            sign_on,
        } = def
        else {
            return None;
        };
        let (ore, filler) = (r.block(ore), r.block(filler));
        let low = (*min_y).clamp(1, WORLD_HEIGHT - 1);
        let high = (*max_y).clamp(1, WORLD_HEIGHT - 1);
        Some(Self {
            ore,
            deep_ore: deep_ore.as_ref().map_or(ore, |id| r.block(id)),
            filler,
            deep_filler: deep_filler.as_ref().map_or(filler, |id| r.block(id)),
            ore_share: ore_share.clamp(0.0, 1.0),
            length: bounded(*length, 8, MAX_LENGTH),
            thickness: bounded(*thickness, 2, MAX_THICKNESS),
            min_y: low.min(high),
            max_y: low.max(high),
            cell: (*cell).clamp(64, 4096) as i32,
            chance: chance.clamp(0.0, 1.0),
            sign: sign.as_ref().map(|id| r.block(id)),
            sign_share: sign_share.clamp(0.0, 0.5),
            sign_on: r.blocks(sign_on),
        })
    }

    /// Farthest a block of a vein may lie from its cell's center point, horizontally.
    fn reach(&self) -> i32 {
        let radius = f64::from(self.thickness.1) / 2.0 * (1.0 + WOBBLE);
        (f64::from(self.length.1) / 2.0 + radius).ceil() as i32 + 1
    }
}

fn bounded((a, b): (u32, u32), min: u32, max: u32) -> (u32, u32) {
    (a.min(b).clamp(min, max), a.max(b).clamp(min, max))
}

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
        let mut rng = Rng::for_column(seed, cell, CELL_SALT + feature as u64);
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
        let keep = if flat { [1.0, 0.0, 1.0] } else { [1.0; 3] };
        let d: [f64; 3] = std::array::from_fn(|i| (p[i] - self.start[i]) * keep[i]);
        let axis: [f64; 3] = std::array::from_fn(|i| self.axis[i] * keep[i]);
        let span = self.length * dot(axis, axis);
        let t = if span > 0.0 {
            (dot(d, axis) / span).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let off: [f64; 3] = std::array::from_fn(|i| d[i] - axis[i] * self.length * t);
        (t, dot(off, off).sqrt())
    }

    /// Radius at `t`: tapers towards both ends.
    fn lens(&self, t: f64) -> f64 {
        let from_middle = 2.0 * t - 1.0;
        self.radius * (0.3 + 0.7 * (1.0 - from_middle * from_middle).sqrt())
    }

    fn y_range(&self) -> (i32, i32) {
        let end = self.start[1] + self.axis[1] * self.length;
        let reach = self.radius * (1.0 + WOBBLE);
        let low = (self.start[1].min(end) - reach).floor() as i32;
        let high = (self.start[1].max(end) + reach).ceil() as i32;
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
    ) -> Vec<Vein> {
        let mut veins = Vec::new();
        for &feature in &content.terrain.deposits {
            let Some(Feature::Deposit(deposit)) = content.features.get(feature) else {
                continue;
            };
            let (reach, side) = (deposit.reach(), deposit.cell);
            let cells = |low: i32, high: i32| {
                (low - reach).div_euclid(side)..=(high + reach).div_euclid(side)
            };
            for cz in cells(min.1, max.1) {
                for cx in cells(min.0, max.0) {
                    let cell = ColumnPos { x: cx, z: cz };
                    let near = |v: &Vein| {
                        (min.0 - reach..=max.0 + reach).contains(&v.center.x)
                            && (min.1 - reach..=max.1 + reach).contains(&v.center.z)
                    };
                    veins.extend(Vein::in_cell(seed, feature, deposit, cell).filter(near));
                }
            }
        }
        veins
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
        let (low, high) = self.vein.y_range();
        let widest = self.vein.radius * (1.0 + WOBBLE);
        let (d, salt) = (self.deposit, ORE_SALT + self.vein.feature as u64);
        for lz in 0..16 {
            for lx in 0..16 {
                let (x, z) = (self.origin.0 + lx, self.origin.1 + lz);
                for y in low..=high {
                    let host = column.get(lx as u32, y, lz as u32);
                    let deep = match host {
                        h if h == terrain.stone => false,
                        h if h == terrain.deep_stone => true,
                        _ => continue,
                    };
                    let p = [x, y, z].map(f64::from);
                    let (t, distance) = self.vein.nearest(p, false);
                    if distance > widest {
                        continue;
                    }
                    let swell = 1.0 + WOBBLE * wobble.at_3d(p[0], p[1], p[2]);
                    if distance > self.vein.lens(t) * swell {
                        continue;
                    }
                    let ore = roll(self.seed, (x, y, z), salt) < d.ore_share;
                    let block = match (ore, deep) {
                        (true, false) => d.ore,
                        (true, true) => d.deep_ore,
                        (false, false) => d.filler,
                        (false, true) => d.deep_filler,
                    };
                    column.set(lx as u32, y, lz as u32, block);
                }
            }
        }
    }

    /// A few ground blocks right above the vein's footprint, never under water.
    fn signs(&self, column: &mut Column, tops: &[[i32; 16]; 16]) {
        let Some(sign) = self.deposit.sign else {
            return;
        };
        let salt = SIGN_SALT + self.vein.feature as u64;
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

/// Uniform in `[0, 1)`, from a block position.
fn roll(seed: Seed, (x, y, z): (i32, i32, i32), salt: u64) -> f64 {
    let salt = salt.wrapping_add((y as u32 as u64) << 16);
    Rng::for_column(seed, ColumnPos { x, z }, salt).unit()
}
