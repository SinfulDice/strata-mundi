//! Upgrades as written in `upgrades/<name>.ron` and the anvil's rules in `anvil.ron`.

use crate::crafting::IngredientDef;
use crate::id::Identifier;
use serde::{Deserialize, Serialize};

/// An upgrade put in one of an item's slots at the anvil, level after level.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Upgrade", deny_unknown_fields)]
pub struct UpgradeDef {
    /// The slot family it goes in (`upgrade_slots/`).
    pub slot: Identifier,
    /// Items it suits; any equipment when empty.
    #[serde(default)]
    pub applies_to: Vec<Target>,
    /// Level 1 first; each replaces the one before.
    pub levels: Vec<LevelDef>,
    /// Upgrades the item must hold first.
    #[serde(default)]
    pub requires: Vec<Identifier>,
    /// Upgrades it cannot share an item with.
    #[serde(default)]
    pub excludes: Vec<Identifier>,
}

/// What a level gives and costs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Level", deny_unknown_fields)]
pub struct LevelDef {
    pub effects: Vec<Effect>,
    /// Experience paid at the anvil (free until experience exists, milestone 9).
    #[serde(default)]
    pub xp: u32,
    /// Laid in the anvil's second cell, `count` of them.
    pub material: IngredientDef,
    #[serde(default = "one")]
    pub count: u16,
}

fn one() -> u16 {
    1
}

/// Kinds of items an upgrade suits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Target {
    Tool,
    Weapon,
    Armor,
}

/// One brick of what an upgrade does. Two upgrades giving the same brick do not add up:
/// the strongest counts.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum Effect {
    /// Weapon damage, in percent.
    Damage(f32),
    /// Breaking speed of a tool, in percent.
    MiningSpeed(f32),
    /// Uses of a new one, in percent.
    Durability(f32),
    /// Protection points of a worn piece.
    Protection(u16),
    /// Share of the armour's burden (speed, stamina, mana) taken off, in percent.
    Lighten(f32),
    /// Unlocks breaking this area at once (a miner's hammer), width × height.
    Area(u8, u8),
}

impl Effect {
    /// Bricks of the same kind do not add up.
    pub fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// Width (across) and height (up, or ahead when hitting a floor) of a hammer's area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AreaSize {
    pub width: u8,
    pub height: u8,
}

impl Default for AreaSize {
    fn default() -> Self {
        Self::SINGLE
    }
}

impl AreaSize {
    pub const SINGLE: Self = Self {
        width: 1,
        height: 1,
    };

    pub fn cells(self) -> u16 {
        u16::from(self.width) * u16::from(self.height)
    }
}

/// `anvil.ron` at the root of a pack: repairing and the growing cost of each repair.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename = "Anvil", deny_unknown_fields, default)]
pub struct AnvilRules {
    /// Share of the uses one repair material gives back.
    pub material_share: f32,
    /// Two like items joined: their uses added, plus this share of a new one's.
    pub combine_bonus: f32,
    /// Experience of a first repair…
    pub repair_xp: u32,
    /// … and added for each repair the item had before…
    pub repair_xp_step: u32,
    /// … up to this.
    pub max_repair_xp: u32,
}

impl Default for AnvilRules {
    fn default() -> Self {
        Self {
            material_share: 0.25,
            combine_bonus: 0.12,
            repair_xp: 1,
            repair_xp_step: 1,
            max_repair_xp: 10,
        }
    }
}

impl AnvilRules {
    pub(super) fn clamped(mut self) -> Self {
        self.material_share = share(self.material_share, 0.01);
        self.combine_bonus = share(self.combine_bonus, 0.0);
        self.max_repair_xp = self.max_repair_xp.max(self.repair_xp);
        self
    }

    /// Experience of a repair after `repairs` earlier ones.
    pub fn repair_cost(&self, repairs: u32) -> u32 {
        let grown = self.repair_xp_step.saturating_mul(repairs);
        self.repair_xp.saturating_add(grown).min(self.max_repair_xp)
    }
}

fn share(value: f32, min: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, 1.0)
    } else {
        min
    }
}
