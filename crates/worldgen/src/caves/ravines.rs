//! Ravines, like Minecraft's canyons: rare, narrow and deep cracks with nearly vertical
//! walls. The world is cut into square regions of `ravine_spacing` blocks, each holding
//! one gently curved ravine whose ends taper off. Some open at the surface, others stay
//! hidden deep underground.

use crate::Seed;
use crate::content::Caves as Settings;
use crate::rng::Rng;
use std::f64::consts::{PI, TAU};
use strata_core::coords::ColumnPos;

const SALT: u64 = 2600;
/// Width at the bottom, as a share of the width at the top.
const BOTTOM_WIDTH: f64 = 0.3;
const HIDDEN_SHARE: f64 = 0.35;
/// Shortest and longest ravine, as a share of the region side.
const LENGTH: (f64, f64) = (0.25, 0.45);
/// Largest sideways bend at the middle, as a share of the length.
const BEND: f64 = 0.2;
/// Straight pieces of the curved center line.
const PIECES: usize = 12;
/// Ledges on the walls: each layer is up to this much narrower.
const ROUGHNESS: f64 = 0.15;
/// Widest ravine, as a share of the `ravines` setting.
const MAX_WIDTH: f64 = 1.25;

pub struct Ravines {
    seed: Seed,
    spacing: i32,
    width: f64,
    depths: (i32, i32),
}

/// Ravines near an area.
pub struct Nearby(Vec<Ravine>);

struct Ravine {
    line: [(f64, f64); PIECES + 1],
    /// Half-width in the middle, in blocks.
    width: f64,
    depth: i32,
    hidden: bool,
    salt: u64,
    /// `(min x, min z, max x, max z)` of the center line.
    bounds: (f64, f64, f64, f64),
}

/// Nearest ravine to one block column, if any.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    /// Horizontal distance to the ravine's edge in blocks, negative inside.
    gap: f64,
    /// 0 on the center line, 1 on the edge.
    distance: f64,
    /// `None` for a ravine that opens at the surface (its top is the tunnel roof).
    top: Option<i32>,
    depth: i32,
    salt: u64,
}

impl Ravines {
    pub fn new(seed: Seed, settings: &Settings) -> Self {
        Self {
            seed,
            spacing: settings.ravine_spacing,
            width: settings.ravines,
            depths: settings.ravine_depth,
        }
    }

    /// The ravines that may cross the blocks from `min` to `max` (inclusive), found once
    /// for all of them.
    pub fn around(&self, min: (i32, i32), max: (i32, i32)) -> Nearby {
        if self.width <= 0.0 {
            return Nearby(Vec::new());
        }
        let region = |c: i32| c.div_euclid(self.spacing);
        let area = (
            (f64::from(min.0), f64::from(min.1)),
            (f64::from(max.0) + 1.0, f64::from(max.1) + 1.0),
        );
        let regions = (region(min.1) - 1..=region(max.1) + 1)
            .flat_map(|rz| (region(min.0) - 1..=region(max.0) + 1).map(move |rx| (rx, rz)));
        Nearby(regions.filter_map(|r| self.ravine(r, area)).collect())
    }

    pub fn span(&self, x: i32, z: i32) -> Span {
        self.around((x, z), (x, z)).span(x, z)
    }

    /// The ravine of a region, if it may come near `area` (`(min, max)` corners).
    fn ravine(&self, (rx, rz): (i32, i32), area: ((f64, f64), (f64, f64))) -> Option<Ravine> {
        let mut rng = Rng::for_column(self.seed, ColumnPos { x: rx, z: rz }, SALT);
        let side = f64::from(self.spacing);
        let start = (
            (f64::from(rx) + rng.unit()) * side,
            (f64::from(rz) + rng.unit()) * side,
        );
        let angle = rng.unit() * TAU;
        let length = side * (LENGTH.0 + (LENGTH.1 - LENGTH.0) * rng.unit());
        let reach = length * (1.0 + BEND) + self.width * 2.0 * MAX_WIDTH;
        let ((ax, az), (bx, bz)) = area;
        let far = start.0 + reach < ax
            || start.0 - reach > bx
            || start.1 + reach < az
            || start.1 - reach > bz;
        if far {
            return None;
        }
        let bend = (rng.unit() * 2.0 - 1.0) * BEND * length;
        let (along, across) = ((angle.cos(), angle.sin()), (-angle.sin(), angle.cos()));
        let line = std::array::from_fn(|i| {
            let t = i as f64 / PIECES as f64;
            let side = bend * (PI * t).sin();
            (
                start.0 + along.0 * length * t + across.0 * side,
                start.1 + along.1 * length * t + across.1 * side,
            )
        });
        let (shallow, deep) = self.depths;
        Some(Ravine {
            width: self.width * (MAX_WIDTH - 0.5 + 0.5 * rng.unit()),
            depth: rng.range(shallow, deep),
            hidden: rng.unit() < HIDDEN_SHARE,
            salt: rng.next_u64(),
            bounds: bounds(&line),
            line,
        })
    }
}

impl Nearby {
    pub fn span(&self, x: i32, z: i32) -> Span {
        let point = (f64::from(x) + 0.5, f64::from(z) + 0.5);
        self.0
            .iter()
            .filter_map(|ravine| ravine.span(point))
            .min_by(|a, b| a.gap.total_cmp(&b.gap))
            .unwrap_or(Span::NONE)
    }
}

fn bounds(line: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    line.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(ax, az, bx, bz), &(x, z)| (ax.min(x), az.min(z), bx.max(x), bz.max(z)),
    )
}

impl Ravine {
    /// The ravine seen from `point`, if `point` is no farther than its width from it.
    fn span(&self, point: (f64, f64)) -> Option<Span> {
        let (ax, az, bx, bz) = self.bounds;
        let reach = self.width * 2.0;
        let outside = point.0 < ax - reach
            || point.0 > bx + reach
            || point.1 < az - reach
            || point.1 > bz + reach;
        if outside {
            return None;
        }
        let (distance, t) = self.nearest(point);
        // Ends taper to a point and rise.
        let taper = (PI * t).sin().max(0.0).sqrt();
        let width = self.width * taper;
        let depth = (f64::from(self.depth) * (0.5 + 0.5 * taper)) as i32;
        Some(Span {
            gap: distance - width,
            distance: if width > 0.0 {
                distance / width
            } else {
                f64::INFINITY
            },
            top: self.hidden.then_some(self.depth),
            depth,
            salt: self.salt,
        })
    }

    /// Distance to the center line and position along it, from 0 to 1.
    fn nearest(&self, (px, pz): (f64, f64)) -> (f64, f64) {
        let mut best = (f64::INFINITY, 0.0);
        for (i, pair) in self.line.windows(2).enumerate() {
            let ((x0, z0), (x1, z1)) = (pair[0], pair[1]);
            let (dx, dz) = (x1 - x0, z1 - z0);
            let squared = dx * dx + dz * dz;
            let along = if squared > 0.0 {
                (((px - x0) * dx + (pz - z0) * dz) / squared).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let distance = (px - x0 - along * dx).hypot(pz - z0 - along * dz);
            if distance < best.0 {
                best = (distance, (i as f64 + along) / PIECES as f64);
            }
        }
        best
    }
}

impl Span {
    const NONE: Self = Self {
        gap: f64::INFINITY,
        distance: f64::INFINITY,
        top: None,
        depth: 0,
        salt: 0,
    };

    pub fn is_inside(&self) -> bool {
        self.distance < 1.0
    }

    /// Whether a ravine opening at the surface is within `margin` blocks.
    pub fn surface_within(&self, margin: f64) -> bool {
        self.top.is_none() && self.gap < margin
    }

    /// Whether the ravine is open at height `y`, below a tunnel roof at `roof`.
    pub fn carves(&self, y: i32, roof: i32) -> bool {
        if self.distance >= 1.0 {
            return false;
        }
        let top = self.top.map_or(roof, |top| top.min(roof));
        let below_top = top - y;
        if !(0..=self.depth).contains(&below_top) {
            return false;
        }
        // Nearly vertical walls that only close in near the bottom.
        let narrowing = (f64::from(below_top) / f64::from(self.depth.max(1))).powi(4);
        let ledge = Rng::from_state(self.salt ^ (y as u64).wrapping_mul(0x9E37_79B9)).unit();
        let width = (1.0 - (1.0 - BOTTOM_WIDTH) * narrowing) * (1.0 - ROUGHNESS * ledge);
        self.distance < width
    }
}
