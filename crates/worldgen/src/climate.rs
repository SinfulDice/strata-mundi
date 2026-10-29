//! Large-scale shape of the world: climate maps, terrain height and biome choice.
//! Everything is a pure function of the seed and the horizontal position.

use crate::Seed;
use crate::content::{Biome, WorldgenContent};
use crate::defs::Climate;
use crate::noise::Noise;

pub struct ClimateMaps {
    temperature: Noise,
    humidity: Noise,
    continentalness: Noise,
    erosion: Noise,
    detail: Noise,
    ridges: Noise,
    jitter: Noise,
    sea_level: f64,
}

/// What the terrain looks like at one horizontal position.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub climate: Climate,
    /// Height of the top solid block.
    pub height: i32,
}

/// Highest reachable terrain, below the 512-block ceiling with room to build.
const MAX_HEIGHT: f64 = 420.0;

impl ClimateMaps {
    pub fn new(seed: Seed, content: &WorldgenContent) -> Self {
        let scale = content.terrain.biome_scale;
        Self {
            temperature: Noise::new(seed, 1, 1.0 / (900.0 * scale), 3),
            humidity: Noise::new(seed, 2, 1.0 / (800.0 * scale), 3),
            continentalness: Noise::new(seed, 3, 1.0 / (1100.0 * scale), 4),
            erosion: Noise::new(seed, 4, 1.0 / (700.0 * scale), 3),
            detail: Noise::new(seed, 5, 1.0 / 64.0, 4),
            ridges: Noise::new(seed, 7, 1.0 / 420.0, 3),
            jitter: Noise::new(seed, 6, 1.0 / 6.0, 1),
            sea_level: f64::from(content.terrain.sea_level),
        }
    }

    pub fn sample(&self, x: i32, z: i32) -> Sample {
        let (fx, fz) = (f64::from(x), f64::from(z));
        let climate = Climate {
            temperature: self.temperature.at(fx, fz),
            humidity: self.humidity.at(fx, fz),
            continentalness: self.continentalness.at(fx, fz),
            erosion: self.erosion.at(fx, fz),
        };
        let height = self.height(&climate, fx, fz);
        Sample { climate, height }
    }

    /// Smooth everywhere: biomes change what covers the ground, not its shape, so
    /// borders never show cliffs.
    fn height(&self, climate: &Climate, x: f64, z: f64) -> i32 {
        let c = climate.continentalness;
        let base = self.sea_level + continental_offset(c);
        let ruggedness = smoothstep(0.0, 0.8, -climate.erosion);
        let inland = smoothstep(-0.1, 0.35, c);
        // Ridged noise (sharp crests, wide valleys) shapes the mountain ranges.
        // Wide ranges with rounded shoulders: a crest climbs over ~100 blocks, not 20.
        let ridge = (1.0 - self.ridges.at(x, z).abs()).powi(2);
        let mountains = ruggedness * inland * (50.0 + 140.0 * ridge);
        let hills = (1.0 - ruggedness) * inland * 14.0;
        let detail = self.detail.at(x, z) * (4.0 + mountains * 0.05 + hills * 0.6);
        (base + mountains + detail + hills * (self.detail.at(z, x) * 0.5 + 0.5))
            .clamp(1.0, MAX_HEIGHT) as i32
    }

    /// Nearest biome in climate space. A little per-block jitter dithers the borders.
    pub fn biome<'a>(
        &self,
        content: &'a WorldgenContent,
        x: i32,
        z: i32,
        sample: &Sample,
    ) -> &'a Biome {
        let jitter = self.jitter.at(f64::from(x), f64::from(z)) * 0.025;
        let c = &sample.climate;
        let distance = |b: &Biome| {
            let t = &b.climate;
            (c.temperature + jitter - t.temperature).powi(2)
                + (c.humidity - jitter - t.humidity).powi(2)
                + 3.0 * (c.continentalness + jitter - t.continentalness).powi(2)
                + (c.erosion - t.erosion).powi(2)
        };
        let biomes = content.biomes();
        // `WorldgenContent::load` never returns an empty biome list.
        biomes
            .iter()
            .min_by(|a, b| distance(a).total_cmp(&distance(b)))
            .unwrap_or(&biomes[0])
    }
}

/// Deep ocean far below sea level, gentle coasts, inland plateaus a few blocks above it.
fn continental_offset(c: f64) -> f64 {
    const POINTS: [(f64, f64); 6] = [
        (-1.0, -45.0),
        (-0.45, -28.0),
        (-0.2, -6.0),
        (-0.08, 2.0),
        (0.3, 9.0),
        (1.0, 24.0),
    ];
    let index = POINTS
        .iter()
        .position(|&(x, _)| x > c)
        .unwrap_or(POINTS.len());
    match index {
        0 => POINTS[0].1,
        i if i == POINTS.len() => POINTS[i - 1].1,
        i => {
            let (x0, y0) = POINTS[i - 1];
            let (x1, y1) = POINTS[i];
            y0 + (y1 - y0) * smoothstep(x0, x1, c)
        }
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
