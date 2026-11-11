use serde::Deserialize;

/// Where a biome sits in climate space; the nearest biome wins.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Climate {
    pub temperature: f64,
    pub humidity: f64,
    pub continentalness: f64,
    pub erosion: f64,
}

/// Size of the shapes of each climate map, in blocks, before the biome size setting:
/// larger temperature and humidity shapes give larger biomes, larger continentalness
/// shapes larger continents and oceans.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClimateSizes {
    pub temperature: f64,
    pub humidity: f64,
    pub continentalness: f64,
    pub erosion: f64,
}

impl Default for ClimateSizes {
    fn default() -> Self {
        Self {
            temperature: 900.0,
            humidity: 800.0,
            continentalness: 1100.0,
            erosion: 700.0,
        }
    }
}

impl ClimateSizes {
    pub(crate) fn bounded(self) -> Self {
        let size = |value: f64| value.clamp(16.0, 65536.0);
        Self {
            temperature: size(self.temperature),
            humidity: size(self.humidity),
            continentalness: size(self.continentalness),
            erosion: size(self.erosion),
        }
    }
}
