//! The survival gauges as the HUD shows them, and the harm the player takes.

use serde::{Deserialize, Serialize};
use strata_core::id::Identifier;

/// Health, needs and effects of the player, sent when they change.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VitalsView {
    pub health: f32,
    pub max_health: f32,
    /// `None` when the world turns hunger off (the bar is hidden).
    pub food: Option<f32>,
    pub max_food: f32,
    pub saturation: f32,
    pub thirst: Option<f32>,
    pub max_thirst: f32,
    pub stamina: Option<f32>,
    pub max_stamina: f32,
    /// Ticks of breath left and at most: shown only under water or while it refills.
    pub breath: u16,
    pub max_breath: u16,
    /// Sprinting allowed (the client's prediction walks otherwise).
    pub can_sprint: bool,
    pub effects: Vec<EffectView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectView {
    pub effect: Identifier,
    pub seconds_left: u32,
    pub harmful: bool,
}

/// The player was hurt: the screen flashes and shakes (when the kind does), a sound plays.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HurtView {
    pub kind: Identifier,
    pub amount: f32,
    pub shakes: bool,
}
