//! A body's temperature: the felt one, following the surroundings slowly, its zone (with
//! a margin before leaving it), how wet it is, and the harm of the extreme zones.

use super::TemperatureZone;
use super::rules::TemperatureRules;
use crate::content::TICKS_PER_SECOND;
use crate::id::Identifier;
use serde::{Deserialize, Serialize};

/// Saved with the player: the felt temperature and the wetness.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BodyTemperature {
    /// `None` until the first sample, which sets it at once.
    #[serde(default)]
    pub felt: Option<f32>,
    /// 1 out of the water, down to 0 as it dries.
    #[serde(default)]
    pub wetness: f32,
    /// The heat of the strongest source nearby, at the last sample (dries faster).
    #[serde(skip)]
    pub heat: f32,
    /// The zone, kept until the temperature leaves it by `Zones::margin`.
    #[serde(skip)]
    pub(crate) zone: Option<TemperatureZone>,
    /// Ticks in the harmful zone toward the next hit.
    #[serde(skip)]
    pub(crate) harm_timer: u16,
}

impl BodyTemperature {
    /// A body already at `felt` (tests, mobs spawned in place).
    pub fn at(felt: f32) -> Self {
        Self {
            felt: Some(felt),
            ..Self::default()
        }
    }

    /// The temperature felt (comfort before the first sample).
    pub fn felt(&self, rules: &TemperatureRules) -> f32 {
        self.felt.unwrap_or_else(|| rules.comfort())
    }

    pub fn zone(&self, rules: &TemperatureRules) -> TemperatureZone {
        self.zone.unwrap_or_else(|| rules.zone(self.felt(rules)))
    }

    /// One tick of water: soaked while in it, drying otherwise (faster near heat).
    pub fn soak(&mut self, in_water: bool, rules: &TemperatureRules) {
        if in_water {
            self.wetness = 1.0;
            return;
        }
        let water = &rules.water;
        let speed = if self.heat > 0.0 {
            water.heat_drying
        } else {
            1.0
        };
        let step = speed / (water.dry_seconds * TICKS_PER_SECOND as f32);
        self.wetness = (self.wetness - step).max(0.0);
    }

    /// A new sample `target`, `ticks` after the last: the felt temperature moves part of
    /// the way (all of it the first time).
    pub fn follow(&mut self, target: f32, ticks: u16, rules: &TemperatureRules) {
        let felt = match self.felt {
            None => target,
            Some(felt) => {
                let seconds = f32::from(ticks) / TICKS_PER_SECOND as f32;
                let share = if rules.smoothing <= 0.0 {
                    1.0
                } else {
                    1.0 - (-seconds / rules.smoothing).exp()
                };
                felt + (target - felt) * share
            }
        };
        self.zone = Some(match self.felt {
            None => rules.zone(felt),
            Some(_) => rules.zone_from(self.zone(rules), felt),
        });
        self.felt = Some(felt);
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
