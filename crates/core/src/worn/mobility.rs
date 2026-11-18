//! `mobility.ron` at the root of a pack: how far the movement abilities equipment gives
//! go (double jump, dash, climbing). Which pieces give them is in their stats.

use super::stats::{WornStat, WornStats};
use crate::physics::{Dash, Mobility};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename = "Mobility", deny_unknown_fields, default)]
pub struct MobilityRules {
    /// Upward speed of a jump in the air, in blocks per second (a ground jump is 8.4).
    pub air_jump_speed: f64,
    /// Jumps in the air at most, whatever the equipment gives (0 to 5).
    pub max_air_jumps: u8,
    /// Speed of a dash, in blocks per second, for `dash_ticks` ticks.
    pub dash_speed: f64,
    pub dash_ticks: u8,
    /// Ticks before the next dash.
    pub dash_cooldown: u16,
    /// Speed up a wall, in blocks per second.
    pub climb_speed: f64,
}

impl Default for MobilityRules {
    fn default() -> Self {
        Self {
            air_jump_speed: 7.6,
            max_air_jumps: 3,
            dash_speed: 14.0,
            dash_ticks: 4,
            dash_cooldown: 30,
            climb_speed: 2.4,
        }
    }
}

impl MobilityRules {
    pub fn clamped(self) -> Self {
        let speed = |value: f64, max: f64| {
            if value.is_finite() {
                value.clamp(0.5, max)
            } else {
                0.5
            }
        };
        Self {
            air_jump_speed: speed(self.air_jump_speed, 12.0),
            max_air_jumps: self.max_air_jumps.min(5),
            dash_speed: speed(self.dash_speed, 20.0),
            dash_ticks: self.dash_ticks.clamp(1, 10),
            dash_cooldown: self.dash_cooldown.clamp(5, 1200),
            climb_speed: speed(self.climb_speed, 6.0),
        }
    }

    /// What the worn stats let the body do.
    pub fn mobility(&self, worn: &WornStats) -> Mobility {
        let most = f32::from(self.max_air_jumps);
        let air_jumps = worn.get(WornStat::AirJumps).round().clamp(0.0, most) as u8;
        let dash = (worn.get(WornStat::Dash) > 0.0).then_some(Dash {
            speed: self.dash_speed,
            ticks: self.dash_ticks,
            cooldown: self.dash_cooldown,
        });
        let climb = (worn.get(WornStat::Climb) > 0.0).then_some(self.climb_speed);
        Mobility {
            air_jumps,
            air_jump_speed: self.air_jump_speed,
            dash,
            climb,
        }
    }
}
