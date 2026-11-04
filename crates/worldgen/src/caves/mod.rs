//! Caves carved after the terrain, like Minecraft's Caves & Cliffs:
//! - spaghetti: long tunnels where two 3D noises are both near zero;
//! - noodles: thin, winding galleries, the same idea at a smaller scale;
//! - cheese: large caverns where one noise is high, more common deep down, with pillars
//!   where a vertical noise rises;
//! - ravines: deep narrow cracks following the zero line of a 2D noise.
//!
//! Inside aquifers, carved space fills with water up to the region's level; near the
//! bottom of the world, with lava. Noise is sampled on a coarse grid and interpolated.

mod aquifers;
mod grid;
mod ravines;

use crate::Seed;
use crate::content::Caves as Settings;
use crate::noise::Noise;
use aquifers::{Aquifers, Table};
use grid::CaveGrid;
use ravines::Ravines;
use std::ops::RangeInclusive;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::SECTION_SIZE;

const LOWEST: i32 = 5;
/// Tunnels are flattened: their noise varies faster along y.
const TUNNEL_STRETCH: f64 = 1.6;
const CAVERN_STRETCH: f64 = 1.8;
const PILLAR_THRESHOLD: f64 = 0.55;

pub struct Caves {
    spaghetti: (Noise, Noise),
    noodles: (Noise, Noise),
    noodle_toggle: Noise,
    cheese: Noise,
    pillars: Noise,
    ravines: Ravines,
    aquifers: Aquifers,
    settings: Settings,
}

/// Highest y each kind of cave may reach under each block, indexed `[z][x]`.
pub struct Roofs {
    /// Tunnels and ravines: up to the surface on dry land, well below it near water.
    pub tunnels: [[i32; 16]; 16],
    /// Caverns never break the surface.
    pub caverns: [[i32; 16]; 16],
}

impl Caves {
    pub fn new(seed: Seed, settings: &Settings) -> Self {
        Self {
            spaghetti: (
                Noise::new(seed, 20, 1.0 / 56.0, 2),
                Noise::new(seed, 21, 1.0 / 56.0, 2),
            ),
            noodles: (
                Noise::new(seed, 22, 1.0 / 26.0, 1),
                Noise::new(seed, 23, 1.0 / 26.0, 1),
            ),
            noodle_toggle: Noise::new(seed, 24, 1.0 / 90.0, 1),
            cheese: Noise::new(seed, 25, 1.0 / 72.0, 2),
            pillars: Noise::new(seed, 29, 1.0 / 14.0, 1),
            ravines: Ravines::new(seed, settings),
            aquifers: Aquifers::new(seed, settings),
            settings: settings.clone(),
        }
    }

    /// Carves the column below `roofs`, never replacing a block of `keep`.
    pub fn carve(&self, column: &mut Column, roofs: &Roofs, keep: &[BlockId]) {
        let top = roofs.tunnels.iter().flatten().copied().max().unwrap_or(0);
        let origin = (column.pos.x * SECTION_SIZE, column.pos.z * SECTION_SIZE);
        let grid = CaveGrid::sample(origin, top, |x, y, z| self.openness(x, y, z));
        for lz in 0..16 {
            for lx in 0..16 {
                let (x, z) = (origin.0 + lx as i32, origin.1 + lz as i32);
                let ravine = self.ravines.span(x, z);
                let table = self.aquifers.table(x, z);
                let (tunnel_roof, cavern_roof) = (roofs.tunnels[lz][lx], roofs.caverns[lz][lx]);
                for y in LOWEST..=tunnel_roof {
                    if table.walls(y) {
                        continue;
                    }
                    let open = grid.at(lx, y, lz);
                    let cave = open > 1.0 || (open > 0.0 && y <= cavern_roof);
                    if !cave && !ravine.carves(y, tunnel_roof) {
                        continue;
                    }
                    let (cx, cz) = (lx as u32, lz as u32);
                    if !keep.contains(&column.get(cx, y, cz)) {
                        column.set(cx, y, cz, self.fill(y, table));
                    }
                }
            }
        }
    }

    /// Whether a tunnel or a surface ravine may open a block of `ys` at `(x, z)`, for
    /// things that must not stand over a cave entrance (trees from other columns, spawn).
    pub fn may_open(&self, x: i32, z: i32, ys: RangeInclusive<i32>) -> bool {
        self.ravines.span(x, z).opens_at_surface()
            || ys
                .into_iter()
                .any(|y| grid::at_point(x, y, z, |x, y, z| self.openness(x, y, z)) > 1.0)
    }

    fn fill(&self, y: i32, table: Table) -> BlockId {
        let s = &self.settings;
        if y <= s.lava_level {
            s.lava
        } else if table.level.is_some_and(|level| y <= level) {
            s.water
        } else {
            BlockId::AIR
        }
    }

    /// Above 1 inside tunnels and galleries (allowed up to the tunnel roof), in `(0, 1]`
    /// inside caverns (allowed up to the cavern roof), 0 or less in solid rock.
    fn openness(&self, x: i32, y: i32, z: i32) -> f64 {
        let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
        let tunnel = self.tunnel(x, y, z);
        if tunnel > 0.0 {
            return 1.0 + tunnel;
        }
        self.cavern(x, y, z)
    }

    fn tunnel(&self, x: f64, y: f64, z: f64) -> f64 {
        let s = &self.settings;
        let ty = y * TUNNEL_STRETCH;
        let near_zero = |noise: &Noise, width: f64| {
            if width <= 0.0 {
                return -1.0;
            }
            1.0 - noise.at_3d(x, ty, z).abs() / width
        };
        let pair =
            |(a, b): &(Noise, Noise), width: f64| near_zero(a, width).min(near_zero(b, width));
        let spaghetti = pair(&self.spaghetti, s.spaghetti);
        let noodle_here = s.noodles > 0.0
            && y <= f64::from(s.noodle_top)
            && self.noodle_toggle.at_3d(x, y, z) > 0.0;
        let noodle = if noodle_here {
            pair(&self.noodles, s.noodles)
        } else {
            -1.0
        };
        spaghetti.max(noodle)
    }

    /// Caverns grow with depth: rare under the surface, common deep down.
    fn cavern(&self, x: f64, y: f64, z: f64) -> f64 {
        let fade = f64::from(self.settings.cavern_top);
        if y > fade * 1.3 {
            return -1.0;
        }
        let height = (y / fade).clamp(0.0, 1.0);
        let threshold = self.settings.cheese + height * height * 0.3;
        let open = self.cheese.at_3d(x, y * CAVERN_STRETCH, z) - threshold;
        if open <= 0.0 {
            return open;
        }
        let pillar = self.pillars.at_3d(x, y * 0.2, z) - PILLAR_THRESHOLD;
        (open - pillar.max(0.0) * 4.0).min(1.0)
    }
}
