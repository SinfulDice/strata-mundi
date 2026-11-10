//! Large-scale shape of the world: climate maps, plain terrain height and biome choice.
//! Everything is a pure function of the seed and the horizontal position.

use crate::Seed;
use crate::content::{Biome, WorldgenContent};
use crate::defs::{Climate, Relief};
use crate::noise::Noise;
use crate::spline::smoothstep;

pub struct ClimateMaps {
    temperature: Noise,
    humidity: Noise,
    continentalness: Noise,
    erosion: Noise,
    weirdness: Noise,
    detail: Noise,
    river: Noise,
    jitter: Noise,
    sea_level: f64,
    relief: Relief,
    /// Height of flat worlds, the same everywhere.
    flat_top: Option<i32>,
}

/// What the terrain looks like at one horizontal position.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub climate: Climate,
    /// Height of the terrain before 3D noise carves overhangs.
    pub height: i32,
    /// 1 in the middle of a river bed, 0 away from rivers.
    pub river: f64,
    /// Amplitude of the 3D noise here, in blocks.
    pub overhang: f64,
}

impl ClimateMaps {
    pub fn new(seed: Seed, content: &WorldgenContent) -> Self {
        let scale = content.terrain.biome_scale;
        let r = &content.terrain.relief;
        Self {
            temperature: Noise::new(seed, 1, 1.0 / (900.0 * scale), 3),
            humidity: Noise::new(seed, 2, 1.0 / (800.0 * scale), 3),
            continentalness: Noise::new(seed, 3, 1.0 / (1100.0 * scale * r.continent_scale), 4),
            erosion: Noise::new(seed, 4, 1.0 / (700.0 * scale), 3),
            weirdness: Noise::new(seed, 7, 1.0 / (r.weirdness_scale * scale), 3),
            detail: Noise::new(seed, 5, 1.0 / 64.0, 4),
            river: Noise::new(seed, 8, 1.0 / (r.river_scale * scale), 2),
            jitter: Noise::new(seed, 6, 1.0 / 6.0, 1),
            sea_level: f64::from(content.terrain.sea_level),
            relief: content.terrain.relief.clone(),
            flat_top: content.terrain.flat.as_ref().map(|flat| flat.top()),
        }
    }

    pub fn sample(&self, x: i32, z: i32) -> Sample {
        let (fx, fz) = (f64::from(x), f64::from(z));
        if let Some(height) = self.flat_top {
            return Sample {
                climate: Climate::default(),
                height,
                river: 0.0,
                overhang: 0.0,
            };
        }
        let climate = Climate {
            temperature: self.temperature.at(fx, fz),
            humidity: self.humidity.at(fx, fz),
            continentalness: self.remap(stretch(self.continentalness.at(fx, fz))),
            erosion: stretch(self.erosion.at(fx, fz)),
        };
        let peaks = peaks_and_valleys(stretch(self.weirdness.at(fx, fz)));
        let land = self.land_height(&climate, peaks, fx, fz);
        let river = self.river_factor(fx, fz, land);
        let height = self.carve_river(land, river);
        let r = &self.relief;
        let inland = r.inland.at(climate.continentalness).clamp(0.0, 1.0);
        let overhang = r.overhang.at(climate.erosion).clamp(0.0, 1.0)
            * inland
            * (1.0 - river)
            * r.overhang_blocks;
        Sample {
            climate,
            height: height.clamp(1.0, r.max_height) as i32,
            river,
            overhang,
        }
    }

    fn remap(&self, continentalness: f64) -> f64 {
        self.relief
            .continentalness
            .as_ref()
            .map_or(continentalness, |curve| {
                curve.at(continentalness).clamp(-1.0, 1.0)
            })
    }

    fn land_height(&self, climate: &Climate, peaks: f64, x: f64, z: f64) -> f64 {
        let land = self.raised_height(climate, peaks, x, z);
        self.sink_wetlands(climate, land, x, z)
    }

    /// Mountains where erosion is low, rolling hills where it is high, terraced plateaus
    /// in between; all of it rising with the distance from the sea.
    fn raised_height(&self, climate: &Climate, peaks: f64, x: f64, z: f64) -> f64 {
        let r = &self.relief;
        let c = climate.continentalness;
        let e = climate.erosion;
        let inland = r.inland.at(c).clamp(0.0, 1.0);
        let rugged = r.rugged.at(e).clamp(0.0, 1.0);
        let relief = rugged * r.peaks.at(peaks) + (1.0 - rugged) * r.hills.at(peaks);
        let detail = self.detail.at(x, z) * (2.0 + 4.0 * rugged + 2.0 * inland);
        let plain = self.sea_level + r.continent.at(c) + inland * relief + detail;
        let plateau = r.plateau.at(e).clamp(0.0, 1.0) * inland;
        if plateau <= 0.0 {
            return plain;
        }
        let raised = plain + plateau * r.plateau_lift;
        let step = r.terrace_step;
        let terraced =
            (raised / step).floor() * step + step * smoothstep(0.8, 1.0, (raised / step).fract());
        raised + (terraced - raised) * smoothstep(0.3, 0.7, plateau)
    }

    /// Humid flat lands sink to a bumpy level near the sea: pools between islets.
    fn sink_wetlands(&self, climate: &Climate, land: f64, x: f64, z: f64) -> f64 {
        let Some(wetlands) = &self.relief.wetlands else {
            return land;
        };
        let wet = wetlands.humidity.at(climate.humidity).clamp(0.0, 1.0)
            * wetlands.erosion.at(climate.erosion).clamp(0.0, 1.0);
        let level = self.sea_level + wetlands.level + self.detail.at(x, z) * wetlands.bumps;
        if wet <= 0.0 || land <= level {
            return land;
        }
        land + (level - land) * wet
    }

    /// Rivers follow the zero line of their own noise and fade out in high terrain.
    fn river_factor(&self, x: f64, z: f64, land: f64) -> f64 {
        let Some(rivers) = &self.relief.rivers else {
            return 0.0;
        };
        let above_sea = land - self.sea_level;
        if above_sea < -2.0 {
            return 0.0;
        }
        let distance = self.river.at(x, z).abs();
        let channel = 1.0 - smoothstep(rivers.width, rivers.width + rivers.banks, distance);
        channel * (1.0 - smoothstep(rivers.highest * 0.6, rivers.highest, above_sea))
    }

    fn carve_river(&self, land: f64, river: f64) -> f64 {
        let Some(rivers) = &self.relief.rivers else {
            return land;
        };
        let bed = self.sea_level - rivers.depth;
        if river <= 0.0 || land <= bed {
            return land;
        }
        land + (bed - land) * smoothstep(0.0, 0.85, river)
    }

    /// Nearest biome in climate space among those whose height and river conditions hold.
    /// A little per-block jitter dithers the borders.
    pub fn biome<'a>(
        &self,
        content: &'a WorldgenContent,
        x: i32,
        z: i32,
        sample: &Sample,
    ) -> &'a Biome {
        let biomes = content.biomes();
        if let Some(forced) = content.terrain.forced_biome.and_then(|i| biomes.get(i)) {
            return forced;
        }
        let jitter = self.jitter.at(f64::from(x), f64::from(z)) * 0.025;
        let c = &sample.climate;
        let distance = |b: &Biome| {
            let t = &b.climate;
            (c.temperature + jitter - t.temperature).powi(2)
                + (c.humidity - jitter - t.humidity).powi(2)
                + 3.0 * (c.continentalness + jitter - t.continentalness).powi(2)
                + (c.erosion - t.erosion).powi(2)
        };
        let threshold = river_biome_threshold(&content.terrain.relief);
        let on_river = sample.river > threshold && sample.height < content.terrain.sea_level;
        let height = sample.height + (jitter * 40.0) as i32;
        let fits = |b: &&Biome| {
            b.river == on_river
                && b.min_height.is_none_or(|min| height >= min)
                && b.max_height.is_none_or(|max| height <= max)
        };
        let nearest = |list: &mut dyn Iterator<Item = &'a Biome>| {
            list.min_by(|a, b| distance(a).total_cmp(&distance(b)))
        };
        // `WorldgenContent::load` never returns an empty biome list.
        nearest(&mut biomes.iter().filter(fits))
            .or_else(|| nearest(&mut biomes.iter()))
            .unwrap_or(&biomes[0])
    }
}

fn river_biome_threshold(relief: &Relief) -> f64 {
    relief.rivers.as_ref().map_or(f64::INFINITY, |r| r.biome)
}

/// Raw fractal noise rarely leaves `[-0.6, 0.6]`; spread it so splines see the full range.
fn stretch(value: f64) -> f64 {
    (value * 1.6).clamp(-1.0, 1.0)
}

/// Folds weirdness so that both of its extremes are peaks and its middle values valleys.
fn peaks_and_valleys(weirdness: f64) -> f64 {
    1.0 - (3.0 * weirdness.abs() - 2.0).abs()
}
