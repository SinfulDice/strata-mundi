//! What the player chooses about the generation of a new world: its type (from the
//! `world_types/` of packs) and a few sliders. Saved in `world.ron`, shared as text codes.

mod code;

pub use code::{CodeError, MAX_CODE_LENGTH};

use crate::coords::WORLD_HEIGHT;
use crate::id::Identifier;
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;

/// Percentages of the pack's values: 100 = unchanged.
pub const BIOME_SIZES: RangeInclusive<u16> = 25..=400;
pub const DENSITIES: RangeInclusive<u16> = 0..=400;
pub const SEA_LEVELS: RangeInclusive<i32> = 48..=240;
pub const MAX_FLAT_LAYERS: usize = 64;
/// Thickness of one flat layer, in blocks (more is several layers of the same block).
pub const LAYER_THICKNESS: RangeInclusive<u16> = 1..=64;
/// Room left above the layers of a flat world, to build.
const FLAT_HEADROOM: i32 = 64;

/// Every value is bounded by `bounded`; the default is the world of the pack, unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldgenSettings {
    /// A `world_types/` entry of the packs.
    pub world_type: Identifier,
    /// Size of biomes and climates, in percent.
    pub biome_size: u16,
    /// `None` = the dimension's sea level; the relief rises and sinks with it.
    pub sea_level: Option<i32>,
    /// Density of caves, in percent (0 = none).
    pub caves: u16,
    /// Frequency of ore veins and deposits, in percent (0 = none).
    pub ores: u16,
    /// Density of structures, in percent; read from v0.6, when structures exist.
    pub structures: u16,
    /// The one biome of types that force one; `None` = the type's own.
    pub biome: Option<Identifier>,
    /// Layers and options of flat types; `None` = the type's own.
    pub flat: Option<FlatSettings>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatSettings {
    /// From the bottom of the world up.
    pub layers: Vec<FlatLayer>,
    pub caves: bool,
    /// Trees and plants of the biome.
    pub features: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlatLayer {
    pub block: Identifier,
    pub thickness: u16,
}

impl Default for WorldgenSettings {
    fn default() -> Self {
        Self {
            world_type: Self::normal(),
            biome_size: 100,
            sea_level: None,
            caves: 100,
            ores: 100,
            structures: 100,
            biome: None,
            flat: None,
        }
    }
}

impl WorldgenSettings {
    pub fn normal() -> Identifier {
        Identifier::builtin("strata", "normal")
    }

    /// The same settings with every value in its range (files and codes can say anything).
    pub fn bounded(mut self) -> Self {
        let clamp =
            |value: u16, range: &RangeInclusive<u16>| value.clamp(*range.start(), *range.end());
        self.biome_size = clamp(self.biome_size, &BIOME_SIZES);
        self.caves = clamp(self.caves, &DENSITIES);
        self.ores = clamp(self.ores, &DENSITIES);
        self.structures = clamp(self.structures, &DENSITIES);
        self.sea_level = self
            .sea_level
            .map(|level| level.clamp(*SEA_LEVELS.start(), *SEA_LEVELS.end()));
        self.flat = self.flat.map(FlatSettings::bounded);
        self
    }

    /// Multiplier of a percentage.
    pub fn factor(percent: u16) -> f64 {
        f64::from(percent) / 100.0
    }
}

impl FlatSettings {
    /// At most `MAX_FLAT_LAYERS` layers of at least one block, leaving room to build.
    pub fn bounded(mut self) -> Self {
        self.layers.truncate(MAX_FLAT_LAYERS);
        let mut room = WORLD_HEIGHT - FLAT_HEADROOM;
        self.layers.retain_mut(|layer| {
            if room <= 0 {
                return false;
            }
            let wanted = layer
                .thickness
                .clamp(*LAYER_THICKNESS.start(), *LAYER_THICKNESS.end());
            let thickness = i32::from(wanted).min(room);
            layer.thickness = thickness as u16;
            room -= thickness;
            true
        });
        self
    }

    /// Height of the highest block of the layers (-1 when there is none).
    pub fn top(&self) -> i32 {
        self.layers
            .iter()
            .map(|layer| i32::from(layer.thickness))
            .sum::<i32>()
            - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(name: &'static str, thickness: u16) -> FlatLayer {
        FlatLayer {
            block: Identifier::builtin("strata", name),
            thickness,
        }
    }

    #[test]
    fn values_are_bounded() {
        let settings = WorldgenSettings {
            biome_size: 0,
            sea_level: Some(5000),
            caves: 9000,
            ores: 401,
            structures: u16::MAX,
            ..WorldgenSettings::default()
        }
        .bounded();
        assert_eq!(settings.biome_size, 25);
        assert_eq!(settings.sea_level, Some(240));
        assert_eq!(
            (settings.caves, settings.ores, settings.structures),
            (400, 400, 400)
        );
        assert_eq!(
            WorldgenSettings::default().bounded(),
            WorldgenSettings::default()
        );
    }

    #[test]
    fn flat_layers_leave_room_to_build() {
        let mut layers = vec![layer("stone", 0)];
        layers.extend(std::iter::repeat_n(layer("dirt", 400), 20));
        let flat = FlatSettings {
            layers,
            caves: false,
            features: false,
        }
        .bounded();
        assert_eq!(flat.layers[0].thickness, 1);
        assert!(
            flat.layers
                .iter()
                .all(|l| LAYER_THICKNESS.contains(&l.thickness))
        );
        assert!(flat.layers.len() < 21, "layers beyond the room are dropped");
        assert_eq!(flat.top(), WORLD_HEIGHT - FLAT_HEADROOM - 1);
        let many = FlatSettings {
            layers: vec![layer("dirt", 1); 100],
            ..flat
        }
        .bounded();
        assert_eq!(many.layers.len(), MAX_FLAT_LAYERS);
    }

    #[test]
    fn old_files_read_as_default() {
        let settings: WorldgenSettings = ron::from_str("()").unwrap();
        assert_eq!(settings, WorldgenSettings::default());
    }
}
