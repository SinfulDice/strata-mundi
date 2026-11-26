//! The felt temperature: what the surroundings come to, as named contributions added up,
//! and how a body's temperature follows it slowly. Free of the world: the server samples
//! the surroundings (`Surroundings`), any body (player or, later, mob) keeps a
//! `BodyTemperature`.
//!
//! Extension point: weather and seasons (v0.5), spells or a hot spring push their own
//! named contribution in `Surroundings::extra`; a whole dimension shifts by
//! `TemperatureRules::dimensions`; potions and spells give the clothing stats as effects.

use super::TemperatureZone;
use super::rules::TemperatureRules;
use crate::content::TICKS_PER_SECOND;
use crate::id::Identifier;
use serde::{Deserialize, Serialize};

/// What the server found around a body at the last sample.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Surroundings {
    pub biome: Option<Identifier>,
    pub dimension: Option<Identifier>,
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
    pub extra: Vec<(Identifier, f32)>,
}

/// Clothing against the weather, in points of the stats of `Clothing`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insulation {
    pub cold: f32,
    pub heat: f32,
}

/// One named part of the felt temperature, in degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct Contribution {
    pub name: Identifier,
    pub degrees: f32,
}

fn part(name: &'static str, degrees: f32) -> Contribution {
    Contribution {
        name: super::super::engine(name),
        degrees,
    }
}

/// The parts of the temperature felt in `around`, soaked to `wetness` (0 to 1), wearing
/// `clothing`. Their sum is the temperature the body goes toward.
pub fn contributions(
    rules: &TemperatureRules,
    around: &Surroundings,
    wetness: f32,
    clothing: Insulation,
) -> Vec<Contribution> {
    let biome = rules.biome(around.biome.as_ref());
    let night = 1.0 - around.daylight.clamp(0.0, 1.0);
    let night_drop = (biome.night - biome.day) * night;
    let mut parts = vec![part("biome", biome.day), part("night", night_drop)];
    let dimension = (around.dimension.as_ref()).and_then(|id| rules.dimensions.get(id));
    if let Some(&shift) = dimension {
        parts.push(part("dimension", shift));
    }
    let above = (around.height as f32 - rules.altitude.from).max(0.0);
    if above > 0.0 {
        parts.push(part("altitude", -above * rules.altitude.per_block));
    }
    if around.sheltered {
        let shelter = &rules.shelter;
        let sun = (biome.day - rules.zones.hot).max(0.0) * (1.0 - night);
        let shade = -night_drop * shelter.night_share - sun * shelter.day_share;
        parts.push(part("shelter", shade));
    }
    let water = &rules.water;
    let soaked =
        water.wet * wetness.clamp(0.0, 1.0) + if around.immersed { water.immersed } else { 0.0 };
    if soaked != 0.0 {
        parts.push(part("water", soaked));
    }
    if around.heat > 0.0 {
        parts.push(part("heat", around.heat));
    }
    parts.extend(around.extra.iter().map(|(name, degrees)| Contribution {
        name: name.clone(),
        degrees: *degrees,
    }));
    let bare: f32 = parts.iter().map(|p| p.degrees).sum();
    let worn = clothing_part(rules, bare, clothing);
    if worn != 0.0 {
        parts.push(part("clothing", worn));
    }
    parts
}

/// Clothing brings `bare` back toward comfort, never past it; warm clothes in the heat
/// make it worse by a share of their warmth.
fn clothing_part(rules: &TemperatureRules, bare: f32, clothing: Insulation) -> f32 {
    let per_point = rules.clothing.per_point;
    let comfort = rules.comfort();
    let warmth = clothing.cold.max(0.0) * per_point;
    let cooling = clothing.heat.max(0.0) * per_point;
    if bare < comfort {
        warmth.min(comfort - bare)
    } else {
        let overheat = warmth * rules.clothing.overheat_share;
        let hotter = bare + overheat;
        overheat - cooling.min((hotter - comfort).max(0.0))
    }
}

/// The sum of the parts.
pub fn total(parts: &[Contribution]) -> f32 {
    parts.iter().map(|p| p.degrees).sum()
}

/// A body's temperature: the felt one (following the surroundings slowly) and how wet it
/// is. Saved with the player.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BodyTemperature {
    /// `None` until the first sample, which sets it at once.
    #[serde(default)]
    pub felt: Option<f32>,
    /// 1 out of the water, down to 0 as it dries.
    #[serde(default)]
    pub wetness: f32,
    /// What the surroundings come to, at the last sample.
    #[serde(skip)]
    pub target: f32,
    /// The heat of the strongest source nearby, at the last sample (dries faster).
    #[serde(skip)]
    pub heat: f32,
    /// Ticks in the harmful zone toward the next hit.
    #[serde(skip)]
    pub(crate) harm_timer: u16,
}

impl BodyTemperature {
    /// The temperature felt (comfort before the first sample).
    pub fn felt(&self, rules: &TemperatureRules) -> f32 {
        self.felt.unwrap_or_else(|| rules.comfort())
    }

    pub fn zone(&self, rules: &TemperatureRules) -> TemperatureZone {
        rules.zone(self.felt(rules))
    }

    /// One tick of water: soaked while in it, drying otherwise (faster near heat).
    pub fn soak(&mut self, in_water: bool, near_heat: bool, rules: &TemperatureRules) {
        if in_water {
            self.wetness = 1.0;
            return;
        }
        let water = &rules.water;
        let speed = if near_heat { water.heat_drying } else { 1.0 };
        let step = speed / (water.dry_seconds * TICKS_PER_SECOND as f32);
        self.wetness = (self.wetness - step).max(0.0);
    }

    /// A new sample `target`, `ticks` after the last: the felt temperature moves part of
    /// the way (all of it the first time).
    pub fn follow(&mut self, target: f32, ticks: u16, rules: &TemperatureRules) {
        self.target = target;
        let Some(felt) = self.felt else {
            self.felt = Some(target);
            return;
        };
        let seconds = f32::from(ticks) / TICKS_PER_SECOND as f32;
        let share = if rules.smoothing <= 0.0 {
            1.0
        } else {
            1.0 - (-seconds / rules.smoothing).exp()
        };
        self.felt = Some(felt + (target - felt) * share);
    }

    /// One tick of harm: the damage kind and amount when a harmful zone hits now.
    pub fn harm_tick(&mut self, rules: &TemperatureRules) -> Option<(Identifier, f32)> {
        let Some(harm) = rules.harm(self.zone(rules)) else {
            self.harm_timer = 0;
            return None;
        };
        self.harm_timer += 1;
        if self.harm_timer < harm.every {
            return None;
        }
        self.harm_timer = 0;
        Some((harm.damage.clone(), harm.amount))
    }

    /// Values read from a save, within their bounds.
    pub fn bounded(mut self) -> Self {
        self.felt = self.felt.filter(|felt| felt.is_finite());
        self.wetness = if self.wetness.is_finite() {
            self.wetness.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }
}
