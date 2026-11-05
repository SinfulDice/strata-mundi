//! Fire and what it burns, as written in the pack.

use serde::Deserialize;

/// Oldest a fire can get: it then burns out (or keeps burning on its fuel).
pub const MAX_FIRE_AGE: u8 = 15;

/// `flammability` of a block, with Minecraft's scale: `catch` is how readily a fire
/// starts in the air next to it, `burn` how readily the block itself catches and burns
/// away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flammability {
    pub catch: u8,
    pub burn: u8,
}

/// `fire` of a block: makes it a fire that ages, spreads to flammable blocks and dies
/// out without fuel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireDef {
    /// Least ticks between two steps; up to 9 more are added, different for each place.
    #[serde(default = "default_tick_rate")]
    pub tick_rate: u32,
    /// Age, 1 to 15, at which a fire without fuel beneath may burn out.
    #[serde(default = "default_max_age")]
    pub max_age: u8,
}

impl FireDef {
    pub fn clamped(self) -> Self {
        Self {
            tick_rate: self.tick_rate.max(1),
            max_age: self.max_age.clamp(1, MAX_FIRE_AGE),
        }
    }
}

fn default_tick_rate() -> u32 {
    30
}

fn default_max_age() -> u8 {
    MAX_FIRE_AGE
}
