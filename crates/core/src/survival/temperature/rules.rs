//! `temperature.ron`: the felt temperature, in degrees Celsius, as a sum of named
//! contributions (biome, night, altitude, water, heat, shelter, clothing), the zones it
//! falls into and what each zone does. The last pack that has the file wins; without one,
//! the values below apply.

use super::TemperatureZone;
use crate::id::Identifier;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Temperature", deny_unknown_fields)]
pub struct TemperatureRules {
    /// Ticks between two samples of the surroundings (the costly part).
    pub sample_every: u16,
    /// Seconds for the felt temperature to cover about two thirds of the way to the
    /// surroundings' (0: at once).
    pub smoothing: f32,
    pub zones: Zones,
    /// Degrees shown beyond the harmful zones at both ends of the HUD gauge.
    pub gauge_span: f32,
    /// By day and by night, for a biome the list does not name.
    pub default_biome: BiomeTemperature,
    pub biomes: BTreeMap<Identifier, BiomeTemperature>,
    /// Added in a whole dimension (the extreme dimensions of v0.10).
    #[serde(default)]
    pub dimensions: BTreeMap<Identifier, f32>,
    pub altitude: Altitude,
    pub water: Water,
    pub shelter: Shelter,
    pub clothing: Clothing,
    pub cold: Strain,
    pub hot: Strain,
    pub freezing: ZoneHarm,
    pub scorching: ZoneHarm,
}

/// Thresholds between the zones: below `freezing` it hurts, below `cold` it strains, up
/// to `hot` it is comfortable, above `scorching` it hurts.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zones {
    pub freezing: f32,
    pub cold: f32,
    pub hot: f32,
    pub scorching: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiomeTemperature {
    pub day: f32,
    pub night: f32,
}

/// Colder in height: `per_block` degrees lost each block above `from`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Altitude {
    pub from: f32,
    pub per_block: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Water {
    /// Degrees while soaked (times the wetness, from 1 to 0 as one dries).
    pub wet: f32,
    /// More with the head under water.
    pub immersed: f32,
    /// Seconds to dry out of the water.
    pub dry_seconds: f32,
    /// Times faster near a source of heat.
    pub heat_drying: f32,
}

/// A roof over the head (a solid block up to `height` blocks above) takes away a share of
/// the night's cold and of the day's heat beyond comfort.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shelter {
    pub height: u8,
    pub night_share: f32,
    pub day_share: f32,
}

/// What is worn (or given by an effect) against the weather: each point of a stat brings
/// the felt temperature `per_point` degrees back toward comfort, never past it. Warm
/// clothes in the heat add `overheat_share` of their warmth.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clothing {
    pub cold_stat: Identifier,
    pub heat_stat: Identifier,
    pub per_point: f32,
    pub overheat_share: f32,
}

/// What a zone does: speed of movement times `speed`, exhaustion added each tick to
/// hunger and thirst, stamina coming back times `stamina_regen`.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strain {
    pub speed: f32,
    pub hunger: f32,
    pub thirst: f32,
    pub stamina_regen: f32,
}

impl Strain {
    pub const NONE: Self = Self {
        speed: 1.0,
        hunger: 0.0,
        thirst: 0.0,
        stamina_regen: 1.0,
    };
}

/// Damage of `damage` every `every` ticks in a harmful zone.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneHarm {
    pub damage: Identifier,
    pub amount: f32,
    pub every: u16,
}

impl TemperatureRules {
    pub fn zone(&self, felt: f32) -> TemperatureZone {
        let z = &self.zones;
        if felt < z.freezing {
            TemperatureZone::Freezing
        } else if felt < z.cold {
            TemperatureZone::Cold
        } else if felt <= z.hot {
            TemperatureZone::Comfortable
        } else if felt <= z.scorching {
            TemperatureZone::Hot
        } else {
            TemperatureZone::Scorching
        }
    }

    /// The middle of the comfortable zone, where clothing brings one back to.
    pub fn comfort(&self) -> f32 {
        (self.zones.cold + self.zones.hot) / 2.0
    }

    pub fn strain(&self, zone: TemperatureZone) -> Strain {
        match zone {
            TemperatureZone::Freezing | TemperatureZone::Cold => self.cold,
            TemperatureZone::Comfortable => Strain::NONE,
            TemperatureZone::Hot | TemperatureZone::Scorching => self.hot,
        }
    }

    pub fn harm(&self, zone: TemperatureZone) -> Option<&ZoneHarm> {
        match zone {
            TemperatureZone::Freezing => Some(&self.freezing),
            TemperatureZone::Scorching => Some(&self.scorching),
            _ => None,
        }
    }

    pub fn biome(&self, biome: Option<&Identifier>) -> BiomeTemperature {
        biome
            .and_then(|id| self.biomes.get(id))
            .copied()
            .unwrap_or(self.default_biome)
    }

    /// Where `felt` sits on the HUD gauge, 0 to 1: each zone takes a sixth, the
    /// comfortable one two, so the thresholds are at 1/6, 1/3, 2/3 and 5/6.
    pub fn gauge(&self, felt: f32) -> f32 {
        let z = &self.zones;
        let span = self.gauge_span.max(1.0);
        let points = [
            (z.freezing - span, 0.0),
            (z.freezing, 1.0 / 6.0),
            (z.cold, 2.0 / 6.0),
            (z.hot, 4.0 / 6.0),
            (z.scorching, 5.0 / 6.0),
            (z.scorching + span, 1.0),
        ];
        if !felt.is_finite() || felt <= points[0].0 {
            return 0.0;
        }
        for pair in points.windows(2) {
            let ((t0, g0), (t1, g1)) = (pair[0], pair[1]);
            if felt <= t1 {
                return g0 + (g1 - g0) * (felt - t0) / (t1 - t0).max(f32::EPSILON);
            }
        }
        1.0
    }

    /// Values a pack may not break the game with: ordered zones, positive periods.
    pub fn clamped(mut self) -> Self {
        let finite = |value: f32, fallback: f32| {
            if value.is_finite() { value } else { fallback }
        };
        let z = &mut self.zones;
        z.freezing = finite(z.freezing, -4.0);
        z.cold = finite(z.cold, 8.0).max(z.freezing);
        z.hot = finite(z.hot, 30.0).max(z.cold);
        z.scorching = finite(z.scorching, 42.0).max(z.hot);
        self.sample_every = self.sample_every.clamp(1, 200);
        self.smoothing = finite(self.smoothing, 0.0).clamp(0.0, 600.0);
        self.gauge_span = finite(self.gauge_span, 10.0).clamp(1.0, 100.0);
        self.water.dry_seconds = finite(self.water.dry_seconds, 60.0).clamp(1.0, 3600.0);
        self.water.heat_drying = finite(self.water.heat_drying, 1.0).max(1.0);
        self.shelter.height = self.shelter.height.min(32);
        self.shelter.night_share = finite(self.shelter.night_share, 0.0).clamp(0.0, 1.0);
        self.shelter.day_share = finite(self.shelter.day_share, 0.0).clamp(0.0, 1.0);
        for strain in [&mut self.cold, &mut self.hot] {
            strain.speed = finite(strain.speed, 1.0).clamp(0.1, 1.0);
            strain.hunger = finite(strain.hunger, 0.0).clamp(0.0, 1.0);
            strain.thirst = finite(strain.thirst, 0.0).clamp(0.0, 1.0);
            strain.stamina_regen = finite(strain.stamina_regen, 1.0).clamp(0.0, 1.0);
        }
        for harm in [&mut self.freezing, &mut self.scorching] {
            harm.every = harm.every.max(1);
            harm.amount = finite(harm.amount, 0.0).max(0.0);
        }
        self
    }
}

impl Default for TemperatureRules {
    fn default() -> Self {
        let mild = BiomeTemperature {
            day: 18.0,
            night: 11.0,
        };
        Self {
            sample_every: 20,
            smoothing: 20.0,
            zones: Zones {
                freezing: -4.0,
                cold: 8.0,
                hot: 30.0,
                scorching: 42.0,
            },
            gauge_span: 10.0,
            default_biome: mild,
            biomes: BTreeMap::new(),
            dimensions: BTreeMap::new(),
            altitude: Altitude {
                from: 140.0,
                per_block: 0.06,
            },
            water: Water {
                wet: -6.0,
                immersed: -6.0,
                dry_seconds: 60.0,
                heat_drying: 3.0,
            },
            shelter: Shelter {
                height: 12,
                night_share: 0.5,
                day_share: 0.5,
            },
            clothing: Clothing {
                cold_stat: Identifier::builtin("strata", "insulation"),
                heat_stat: Identifier::builtin("strata", "heat_insulation"),
                per_point: 3.0,
                overheat_share: 0.5,
            },
            cold: Strain {
                speed: 0.85,
                hunger: 0.003,
                thirst: 0.0,
                stamina_regen: 1.0,
            },
            hot: Strain {
                speed: 1.0,
                hunger: 0.0,
                thirst: 0.004,
                stamina_regen: 0.5,
            },
            freezing: ZoneHarm {
                damage: Identifier::builtin("strata", "freezing"),
                amount: 1.0,
                every: 80,
            },
            scorching: ZoneHarm {
                damage: Identifier::builtin("strata", "overheating"),
                amount: 1.0,
                every: 80,
            },
        }
    }
}
