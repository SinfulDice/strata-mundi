//! The felt temperature: what the surroundings come to, as named contributions added up.
//! Free of the world: the server fills `Surroundings`; `target` gives the sum alone,
//! `contributions` the parts (journal, debug).
//!
//! Extension point: weather and seasons (v0.5), spells or a hot spring push their own
//! named contribution in `Surroundings::extra`; a whole dimension shifts by
//! `TemperatureRules::dimensions`; potions and spells give the clothing stats as effects.

use super::rules::TemperatureRules;
use crate::id::Identifier;

/// What the server found around a body at a sample.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Surroundings<'a> {
    pub biome: Option<&'a Identifier>,
    pub dimension: Option<&'a Identifier>,
    /// Height of the feet, in blocks.
    pub height: f64,
    /// 0 at night, 1 by day (`DayCycle::daylight`).
    pub daylight: f32,
    /// A solid block over the head (`Shelter::height`).
    pub sheltered: bool,
    /// The head is under water.
    pub immersed: bool,
    /// Degrees the strongest source of heat nearby gives (0: none).
    pub heat: f32,
    /// More named contributions (weather, seasons, spells), in degrees.
    pub extra: &'a [(Identifier, f32)],
}

/// Clothing against the weather, in points of the stats of `Clothing` (negative points
/// take one further from comfort).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insulation {
    pub cold: f32,
    pub heat: f32,
}

/// The name of a contribution: one of the engine's (`BIOME`…) or one pushed in `extra`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartName<'a> {
    Engine(&'static str),
    Named(&'a Identifier),
}

pub const BIOME: &str = "biome";
pub const NIGHT: &str = "night";
pub const DIMENSION: &str = "dimension";
pub const ALTITUDE: &str = "altitude";
pub const SHELTER: &str = "shelter";
pub const WATER: &str = "water";
pub const HEAT: &str = "heat";
pub const CLOTHING: &str = "clothing";

/// One named part of the felt temperature, in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contribution<'a> {
    pub name: PartName<'a>,
    pub degrees: f32,
}

/// The parts of the temperature felt in `around`, soaked to `wetness` (0 to 1), wearing
/// `clothing`.
pub fn contributions<'a>(
    rules: &TemperatureRules,
    around: &Surroundings<'a>,
    wetness: f32,
    clothing: Insulation,
) -> Vec<Contribution<'a>> {
    let mut parts = Vec::new();
    each_part(rules, around, wetness, clothing, |name, degrees| {
        parts.push(Contribution { name, degrees });
    });
    parts
}

/// The temperature the body goes toward: the sum of the parts, without listing them.
pub fn target(
    rules: &TemperatureRules,
    around: &Surroundings<'_>,
    wetness: f32,
    clothing: Insulation,
) -> f32 {
    let mut sum = 0.0;
    each_part(rules, around, wetness, clothing, |_, degrees| {
        sum += degrees
    });
    sum
}

/// The sum of listed parts.
pub fn total(parts: &[Contribution<'_>]) -> f32 {
    parts.iter().map(|p| p.degrees).sum()
}

fn each_part<'a>(
    rules: &TemperatureRules,
    around: &Surroundings<'a>,
    wetness: f32,
    clothing: Insulation,
    mut visit: impl FnMut(PartName<'a>, f32),
) {
    let mut bare = 0.0;
    let mut add = |name: PartName<'a>, degrees: f32| {
        bare += degrees;
        visit(name, degrees);
    };
    let biome = rules.biome(around.biome);
    let night = 1.0 - around.daylight.clamp(0.0, 1.0);
    let night_drop = (biome.night - biome.day) * night;
    add(PartName::Engine(BIOME), biome.day);
    add(PartName::Engine(NIGHT), night_drop);
    if let Some(&shift) = around.dimension.and_then(|id| rules.dimensions.get(id)) {
        add(PartName::Engine(DIMENSION), shift);
    }
    let above = (around.height as f32 - rules.altitude.from).max(0.0);
    if above > 0.0 {
        add(
            PartName::Engine(ALTITUDE),
            -above * rules.altitude.per_block,
        );
    }
    if around.sheltered {
        let shelter = &rules.shelter;
        let sun = (biome.day - rules.zones.hot).max(0.0) * (1.0 - night);
        let shade = -night_drop * shelter.night_share - sun * shelter.day_share;
        add(PartName::Engine(SHELTER), shade);
    }
    let water = &rules.water;
    let immersed = if around.immersed { water.immersed } else { 0.0 };
    let soaked = water.wet * wetness.clamp(0.0, 1.0) + immersed;
    if soaked != 0.0 {
        add(PartName::Engine(WATER), soaked);
    }
    if around.heat > 0.0 {
        add(PartName::Engine(HEAT), around.heat);
    }
    for (name, degrees) in around.extra {
        add(PartName::Named(name), *degrees);
    }
    let worn = clothing_part(rules, bare, clothing);
    if worn != 0.0 {
        visit(PartName::Engine(CLOTHING), worn);
    }
}

/// Clothing brings `bare` back toward comfort, never past it; warm clothes in the heat
/// make it worse by a share of their warmth. Negative points push away from comfort.
fn clothing_part(rules: &TemperatureRules, bare: f32, clothing: Insulation) -> f32 {
    let per_point = rules.clothing.per_point;
    let comfort = rules.comfort();
    let warmth = clothing.cold * per_point;
    let cooling = clothing.heat * per_point;
    if bare < comfort {
        if warmth < 0.0 {
            warmth
        } else {
            warmth.min(comfort - bare)
        }
    } else {
        let overheat = warmth.max(0.0) * rules.clothing.overheat_share;
        let hotter = bare + overheat;
        if cooling < 0.0 {
            overheat - cooling
        } else {
            overheat - cooling.min((hotter - comfort).max(0.0))
        }
    }
}
