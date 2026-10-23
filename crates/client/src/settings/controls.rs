//! Controls: bindings, sensitivity, gamepad dead zone and movement comforts.

use crate::input::Bindings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlSettings {
    pub auto_jump: bool,
    /// Climb half-block steps (slabs, from a later version) without jumping.
    pub step_up: bool,
    /// Name of the aimed block at the top of the screen.
    pub show_block_name: bool,
    /// Mouse look speed, in percent of the default.
    pub mouse_sensitivity: u32,
    /// Right stick look speed, in percent of the default.
    pub stick_sensitivity: u32,
    pub invert_y: bool,
    /// Stick movements smaller than this are ignored, in percent (drift of worn sticks).
    pub dead_zone: u32,
    pub bindings: Bindings,
}

impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            auto_jump: true,
            step_up: true,
            show_block_name: true,
            mouse_sensitivity: 100,
            stick_sensitivity: 100,
            invert_y: false,
            dead_zone: 15,
            bindings: Bindings::default(),
        }
    }
}

/// Radians per mouse pixel at 100 %.
const MOUSE_RADIANS: f32 = 0.0025;
/// Radians per second at full stick deflection, at 100 %.
const STICK_RADIANS: f32 = 3.0;

impl ControlSettings {
    pub fn mouse_radians(&self) -> f32 {
        MOUSE_RADIANS * self.mouse_sensitivity as f32 / 100.0
    }

    pub fn stick_radians(&self) -> f32 {
        STICK_RADIANS * self.stick_sensitivity as f32 / 100.0
    }

    pub fn dead_zone_fraction(&self) -> f32 {
        self.dead_zone.min(90) as f32 / 100.0
    }
}
