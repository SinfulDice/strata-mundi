//! The sky of each dimension: its day cycle (`day` of `dimensions/<name>.ron`, gameplay)
//! and its look (`skies/<name>.ron`: colors, sun, moon and stars, which resource packs may
//! restyle).

mod cycle;

pub use cycle::{DayCycle, DayCycleDef, MOON_PHASES, Moment};

use crate::content::{TextureId, TextureLibrary};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet};
use crate::texture::{HexColor, Rgba};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Only the day cycle of a dimension file; the terrain is read by the generator.
#[derive(Debug, Deserialize)]
#[serde(rename = "Dimension")]
struct DimensionDays {
    #[serde(default)]
    day: DayCycleDef,
}

/// Zenith and horizon colors at one moment of the day.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gradient {
    pub zenith: HexColor,
    pub horizon: HexColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StarsDef {
    /// Share of the sky's cells holding a star (0 to 0.05).
    pub density: f32,
    /// 0 to 1.
    pub brightness: f32,
    /// How much stars flicker, 0 (steady) to 1.
    pub twinkle: f32,
}

/// `skies/<name>.ron`: named like the dimension it covers.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Sky", deny_unknown_fields)]
pub struct SkyDef {
    pub day: Gradient,
    pub night: Gradient,
    /// Sky at sunrise and sunset, the horizon glowing on the sun's side.
    pub twilight: Gradient,
    pub sun: Identifier,
    pub moon: Identifier,
    /// Apparent half-width, in degrees.
    pub sun_size: f32,
    pub moon_size: f32,
    /// How far the path of the sun and moon leans south, in degrees.
    pub path_tilt: f32,
    pub stars: StarsDef,
}

/// A sky ready to draw: colors in sRGB, textures resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Sky {
    pub day: [Rgba; 2],
    pub night: [Rgba; 2],
    pub twilight: [Rgba; 2],
    pub sun: TextureId,
    pub moon: TextureId,
    /// Half-widths, in radians.
    pub sun_size: f32,
    pub moon_size: f32,
    pub path_tilt: f32,
    pub stars: StarsDef,
}

impl Sky {
    /// Plain blue, used when a dimension has no sky file.
    pub fn fallback() -> Self {
        let blue = Rgba([0x78, 0xa7, 0xff, 255]);
        Self {
            day: [blue; 2],
            night: [Rgba([0x0a, 0x0e, 0x22, 255]); 2],
            twilight: [blue; 2],
            sun: TextureId::MISSING,
            moon: TextureId::MISSING,
            sun_size: 0.0,
            moon_size: 0.0,
            path_tilt: 0.0,
            stars: StarsDef {
                density: 0.0,
                brightness: 0.0,
                twinkle: 0.0,
            },
        }
    }

    fn resolve(def: &SkyDef, textures: &TextureLibrary) -> Self {
        let texture = |id: &Identifier| textures.images.key(id).unwrap_or(TextureId::MISSING);
        let pair = |gradient: &Gradient| [gradient.zenith.0, gradient.horizon.0];
        let angle = |degrees: f32, max: f32| bounded(degrees, 0.0, max).to_radians();
        Self {
            day: pair(&def.day),
            night: pair(&def.night),
            twilight: pair(&def.twilight),
            sun: texture(&def.sun),
            moon: texture(&def.moon),
            sun_size: angle(def.sun_size, 30.0),
            moon_size: angle(def.moon_size, 30.0),
            path_tilt: angle(def.path_tilt, 60.0),
            stars: StarsDef {
                density: bounded(def.stars.density, 0.0, 0.05),
                brightness: bounded(def.stars.brightness, 0.0, 1.0),
                twinkle: bounded(def.stars.twinkle, 0.0, 1.0),
            },
        }
    }
}

fn bounded(value: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

/// Day cycles by dimension, read from the content packs.
pub fn load_days(packs: &PackSet, errors: &mut Vec<PackError>) -> BTreeMap<Identifier, DayCycle> {
    packs
        .layered::<DimensionDays>("dimensions", errors)
        .into_iter()
        .map(|(id, file)| (id, DayCycle::new(&file.value.day)))
        .collect()
}

/// Skies by dimension, from content and resource packs.
pub fn load_skies(
    presentation: &PackSet,
    textures: &TextureLibrary,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, Sky> {
    presentation
        .layered::<SkyDef>("skies", errors)
        .into_iter()
        .map(|(id, file)| (id, Sky::resolve(&file.value, textures)))
        .collect()
}
