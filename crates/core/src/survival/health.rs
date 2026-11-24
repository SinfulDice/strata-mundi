//! Health of any body (the player now, the mobs from v0.4): points, maximum, the short
//! invulnerability after a hit, and how far it has been falling.

use super::rules::{ArmorRules, FallRules};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    /// The base maximum plus what is worn (`strata:max_health`), worked out each tick.
    pub max: f32,
    /// Ticks left during which only a stronger hit hurts (by the difference).
    #[serde(default)]
    pub invulnerable: u16,
    /// The hit that started the invulnerability.
    #[serde(default)]
    pub last_hit: f32,
    /// Highest point of the fall going on (`None` on the ground, swimming, flying).
    #[serde(skip)]
    pub fall_peak: Option<f64>,
}

/// Falling slower than this (blocks per second) is no fall: the apex of a jump, a climb
/// down a wall. The fall counts from where the body went faster.
const FALLING_SPEED: f64 = 3.0;

impl Health {
    pub fn full(max: f32) -> Self {
        Self {
            current: max,
            max,
            invulnerable: 0,
            last_hit: 0.0,
            fall_peak: None,
        }
    }

    /// A new maximum: more room does not heal, less takes away what no longer fits.
    pub fn set_max(&mut self, max: f32) {
        self.max = max.max(1.0);
        self.current = self.current.min(self.max);
    }

    pub fn heal(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current + amount.max(0.0)).min(self.max);
        self.current - before
    }

    /// Takes `amount` (already reduced); returns what was taken. During the invulnerability
    /// only what exceeds the hit that started it counts, unless `piercing` (effects over
    /// time, hunger).
    pub fn hurt(&mut self, amount: f32, piercing: bool, invulnerability: u16) -> f32 {
        if amount.is_nan() || amount <= 0.0 {
            return 0.0;
        }
        let dealt = if piercing || self.invulnerable == 0 {
            amount
        } else {
            amount - self.last_hit
        };
        if dealt <= 0.0 {
            return 0.0;
        }
        if !piercing {
            if self.invulnerable == 0 {
                self.last_hit = amount;
                self.invulnerable = invulnerability;
            } else {
                self.last_hit = amount.max(self.last_hit);
            }
        }
        let before = self.current;
        self.current = (self.current - dealt).max(0.0);
        before - self.current
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn tick(&mut self) {
        self.invulnerable = self.invulnerable.saturating_sub(1);
    }

    /// Follows a fall after one move; returns the blocks fallen when it lands. `cushioned`:
    /// in a liquid, flying or riding, which ends any fall.
    pub fn follow_fall(&mut self, y: f64, falling: f64, on_ground: bool, cushioned: bool) -> f64 {
        if cushioned {
            self.fall_peak = None;
            return 0.0;
        }
        if on_ground {
            return self
                .fall_peak
                .take()
                .map_or(0.0, |peak| (peak - y).max(0.0));
        }
        if falling > -FALLING_SPEED || self.fall_peak.is_none() {
            self.fall_peak = Some(y);
        }
        0.0
    }
}

/// Damage of a fall of `blocks`.
pub fn fall_damage(blocks: f64, rules: &FallRules) -> f32 {
    let beyond = blocks as f32 - rules.safe;
    if beyond <= 0.0 {
        0.0
    } else {
        beyond.ceil() * rules.per_block
    }
}

/// `amount` lessened by armour `protection` (when the kind allows) and by a resistance in
/// percent.
pub fn reduced(amount: f32, protection: Option<u16>, resistance: f32, rules: &ArmorRules) -> f32 {
    let by_armor = protection.map_or(0.0, |points| {
        (f32::from(points) * rules.per_point).clamp(0.0, rules.max)
    });
    let by_resistance = (resistance / 100.0).clamp(0.0, 1.0);
    (amount * (1.0 - by_armor) * (1.0 - by_resistance)).max(0.0)
}
