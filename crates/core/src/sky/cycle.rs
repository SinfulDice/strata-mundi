//! The day and night of a dimension: where the sun is, how much sky light reaches the
//! ground, and the phase of the moon, all as continuous functions of the world time.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Shortest and longest full day a pack may ask for, in ticks (1 minute to 10 hours).
const DAY_LENGTH_RANGE: (u64, u64) = (1_200, 720_000);
/// The day (or the night) never takes less than this share of the cycle.
const MIN_SHARE: f64 = 0.05;
pub const MOON_PHASES: u64 = 8;

/// `day` of `dimensions/<name>.ron`, before bounds are applied.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayCycleDef {
    /// Ticks of a full day and night (24 000 = 20 minutes).
    #[serde(default = "default_length")]
    pub length: u64,
    /// Share of the cycle when the sun is up; the seasons will move it (v0.5).
    #[serde(default = "half")]
    pub day_share: f64,
    /// Share of the cycle each sunrise and sunset takes to brighten or darken the world.
    #[serde(default = "default_twilight")]
    pub twilight: f64,
    /// Sky light at night, as a fraction of the daylight.
    #[serde(default = "default_night_light")]
    pub night_light: f32,
}

impl Default for DayCycleDef {
    fn default() -> Self {
        Self {
            length: default_length(),
            day_share: half(),
            twilight: default_twilight(),
            night_light: default_night_light(),
        }
    }
}

fn default_length() -> u64 {
    24_000
}

fn half() -> f64 {
    0.5
}

fn default_twilight() -> f64 {
    0.06
}

fn default_night_light() -> f32 {
    0.2
}

/// A bounded day cycle. Time 0 is the first sunrise; the world time keeps counting days.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayCycle {
    length: u64,
    day_share: f64,
    twilight: f64,
    night_light: f32,
}

impl Default for DayCycle {
    fn default() -> Self {
        Self::new(&DayCycleDef::default())
    }
}

/// The named moments of `/time set`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    /// Just after sunrise, once the world is fully lit.
    Day,
    Noon,
    /// Just after sunset, once the night has fallen.
    Night,
    Midnight,
}

impl DayCycle {
    pub fn new(def: &DayCycleDef) -> Self {
        let day_share = finite_or(def.day_share, 0.5).clamp(MIN_SHARE, 1.0 - MIN_SHARE);
        let shortest = day_share.min(1.0 - day_share);
        Self {
            length: def.length.clamp(DAY_LENGTH_RANGE.0, DAY_LENGTH_RANGE.1),
            day_share,
            twilight: finite_or(def.twilight, 0.06).clamp(0.001, shortest),
            night_light: finite_or(f64::from(def.night_light), 0.2).clamp(0.0, 1.0) as f32,
        }
    }

    /// The bounded values, as the server sends them to its players.
    pub fn def(&self) -> DayCycleDef {
        DayCycleDef {
            length: self.length,
            day_share: self.day_share,
            twilight: self.twilight,
            night_light: self.night_light,
        }
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn night_light(&self) -> f32 {
        self.night_light
    }

    /// 0 at sunrise, `day_share` at sunset, back to 1 at the next sunrise.
    pub fn fraction(&self, time: f64) -> f64 {
        (time / self.length as f64).rem_euclid(1.0)
    }

    /// Which day `time` belongs to (its night included).
    pub fn day(&self, time: u64) -> u64 {
        time / self.length
    }

    /// Ticks into the current day.
    pub fn time_of_day(&self, time: u64) -> u64 {
        time % self.length
    }

    /// Ticks after sunrise of a named moment.
    pub fn moment(&self, moment: Moment) -> u64 {
        let length = self.length as f64;
        let settled = self.twilight * 0.5;
        let night = 1.0 - self.day_share;
        let fraction = match moment {
            Moment::Day => settled,
            Moment::Noon => self.day_share * 0.5,
            Moment::Night => self.day_share + settled,
            Moment::Midnight => self.day_share + night * 0.5,
        };
        (fraction * length).round() as u64 % self.length
    }

    /// Signed distance to the nearest sunrise or sunset, as a share of the cycle: positive
    /// while the sun is up, negative at night, zero at both. Continuous over the cycle.
    fn daytime_margin(&self, fraction: f64) -> f64 {
        if fraction <= self.day_share {
            fraction.min(self.day_share - fraction)
        } else {
            -(fraction - self.day_share).min(1.0 - fraction)
        }
    }

    /// 0 at night, 1 by day, eased through each sunrise and sunset.
    pub fn daylight(&self, time: f64) -> f32 {
        let half = self.twilight * 0.5;
        smoothstep(-half, half, self.daytime_margin(self.fraction(time))) as f32
    }

    /// How strongly the horizon glows: 1 right at sunrise or sunset, 0 away from them.
    pub fn twilight_glow(&self, time: f64) -> f32 {
        let margin = self.daytime_margin(self.fraction(time)).abs();
        (1.0 - smoothstep(0.0, self.twilight, margin)) as f32
    }

    /// Sky light reaching the world, from `night_light` to 1.
    pub fn sky_factor(&self, time: f64) -> f32 {
        let night = self.night_light;
        night + (1.0 - night) * self.daylight(time)
    }

    /// Angle of the sun along its path: 0 rising in the east, π/2 at noon, π setting in
    /// the west, 3π/2 under the feet at midnight. The day and the night each take half a
    /// turn, whatever their share of the cycle.
    pub fn sun_angle(&self, time: f64) -> f64 {
        let fraction = self.fraction(time);
        if fraction <= self.day_share {
            PI * fraction / self.day_share
        } else {
            PI + PI * (fraction - self.day_share) / (1.0 - self.day_share)
        }
    }

    /// One phase per night, `0` full and `4` new, in a cycle of eight nights. The phase
    /// turns at noon, while the moon is under the horizon, so it never changes in sight.
    pub fn moon_phase(&self, time: u64) -> u8 {
        // The morning before the first noon still shows the night before it (phase 7).
        let lead = self.length * MOON_PHASES - self.moment(Moment::Noon);
        (time.saturating_add(lead) / self.length % MOON_PHASES) as u8
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_factor_is_continuous_and_bounded_over_a_day() {
        let cycle = DayCycle::default();
        let night = cycle.night_light();
        let mut previous = cycle.sky_factor(0.0);
        for tick in 1..=cycle.length() * 2 {
            let factor = cycle.sky_factor(tick as f64);
            assert!((night..=1.0).contains(&factor), "tick {tick}: {factor}");
            assert!((factor - previous).abs() < 0.002, "jump at tick {tick}");
            previous = factor;
        }
        assert_eq!(cycle.sky_factor(cycle.moment(Moment::Noon) as f64), 1.0);
        assert_eq!(
            cycle.sky_factor(cycle.moment(Moment::Midnight) as f64),
            night
        );
    }

    #[test]
    fn uneven_days_stay_continuous() {
        let def = DayCycleDef {
            length: 2_000,
            day_share: 0.7,
            twilight: 0.2,
            night_light: 0.1,
        };
        let cycle = DayCycle::new(&def);
        for tick in 0..cycle.length() * 2 {
            let (a, b) = (tick as f64, tick as f64 + 1.0);
            assert!((cycle.sky_factor(a) - cycle.sky_factor(b)).abs() < 0.02);
            let turn = (cycle.sun_angle(b) - cycle.sun_angle(a)).rem_euclid(2.0 * PI);
            assert!(turn < 0.01, "the sun jumps at tick {tick}");
        }
    }

    #[test]
    fn bad_packs_are_bounded() {
        let def = DayCycleDef {
            length: 0,
            day_share: f64::NAN,
            twilight: 5.0,
            night_light: -3.0,
        };
        let cycle = DayCycle::new(&def);
        assert_eq!(cycle.length(), DAY_LENGTH_RANGE.0);
        assert_eq!(cycle.night_light(), 0.0);
        assert!(cycle.twilight <= 0.5 && cycle.day_share == 0.5);
    }

    #[test]
    fn the_moon_changes_once_a_night_over_eight_nights() {
        let cycle = DayCycle::default();
        let length = cycle.length();
        let midnight = cycle.moment(Moment::Midnight);
        let phases: Vec<u8> = (0..9)
            .map(|day| cycle.moon_phase(day * length + midnight))
            .collect();
        assert_eq!(phases, [0, 1, 2, 3, 4, 5, 6, 7, 0]);
        let night = cycle.moment(Moment::Night);
        assert_eq!(cycle.moon_phase(night), cycle.moon_phase(length - 1));
    }

    #[test]
    fn the_moon_never_changes_phase_in_sight() {
        for day_share in [0.5, 0.2, 0.8] {
            let cycle = DayCycle::new(&DayCycleDef {
                day_share,
                ..DayCycleDef::default()
            });
            for tick in 1..cycle.length() * 3 {
                // The moon faces the sun; its height is at most -sin of the sun's angle.
                let moon_height = -cycle.sun_angle(tick as f64).sin();
                if moon_height > -0.12 {
                    assert_eq!(
                        cycle.moon_phase(tick),
                        cycle.moon_phase(tick - 1),
                        "phase changed in sight at tick {tick} (share {day_share})"
                    );
                }
            }
        }
    }

    #[test]
    fn bounded_values_survive_a_second_pass() {
        let cycle = DayCycle::new(&DayCycleDef {
            length: 5,
            day_share: 0.99,
            twilight: 0.3,
            night_light: 0.4,
        });
        assert_eq!(DayCycle::new(&cycle.def()), cycle);
    }

    #[test]
    fn moments_fall_in_their_part_of_the_day() {
        let cycle = DayCycle::default();
        assert_eq!(cycle.moment(Moment::Noon), 6_000);
        assert_eq!(cycle.moment(Moment::Midnight), 18_000);
        assert_eq!(cycle.daylight(cycle.moment(Moment::Day) as f64), 1.0);
        assert_eq!(cycle.daylight(cycle.moment(Moment::Night) as f64), 0.0);
        assert!(cycle.twilight_glow(0.0) > 0.99 && cycle.twilight_glow(6_000.0) == 0.0);
    }
}
