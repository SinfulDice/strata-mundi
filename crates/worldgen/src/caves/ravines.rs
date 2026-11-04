//! Ravines: long cracks along the zero line of a 2D noise, only where a sparse mask
//! allows them. V-shaped, narrowing with depth. Some open at the surface, others stay
//! hidden deep underground.

use crate::Seed;
use crate::content::Caves as Settings;
use crate::noise::Noise;
use crate::spline::smoothstep;

/// Width at the bottom, as a share of the width at the top.
const BOTTOM_WIDTH: f64 = 0.2;

pub struct Ravines {
    line: Noise,
    mask: Noise,
    depth: Noise,
    width: f64,
}

/// Ravine crossing one block column, if any.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    /// 0 on the ravine's center line, 1 on its edge; 1 or more = no ravine.
    distance: f64,
    /// `None` for a ravine that opens at the surface (its top is the tunnel roof).
    top: Option<i32>,
    depth: i32,
}

impl Ravines {
    pub fn new(seed: Seed, settings: &Settings) -> Self {
        Self {
            line: Noise::new(seed, 26, 1.0 / 170.0, 2),
            mask: Noise::new(seed, 27, 1.0 / 420.0, 1),
            depth: Noise::new(seed, 28, 1.0 / 300.0, 1),
            width: settings.ravines,
        }
    }

    pub fn span(&self, x: i32, z: i32) -> Span {
        let (fx, fz) = (f64::from(x), f64::from(z));
        let strength = smoothstep(0.3, 0.45, self.mask.at(fx, fz).abs());
        let width = self.width * strength;
        if width <= 0.0 {
            return Span::NONE;
        }
        let distance = self.line.at(fx, fz).abs() / width;
        if distance >= 1.0 {
            return Span::NONE;
        }
        let shape = self.depth.at(fx, fz);
        Span {
            distance,
            top: (shape < -0.15).then(|| 30 + (shape.abs() * 50.0) as i32),
            depth: 30 + (shape.abs() * 50.0) as i32,
        }
    }
}

impl Span {
    const NONE: Self = Self {
        distance: f64::INFINITY,
        top: None,
        depth: 0,
    };

    pub fn opens_at_surface(&self) -> bool {
        self.distance < 1.0 && self.top.is_none()
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
        let narrowing = f64::from(below_top) / f64::from(self.depth.max(1));
        self.distance < 1.0 - (1.0 - BOTTOM_WIDTH) * narrowing
    }
}
