use super::slots::{UpgradeSlotFamily, valid_slots};
use crate::content::ToolDef;
use crate::crafting::IngredientDef;
use crate::id::Identifier;
use crate::pack::PackError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

const MAX_DAMAGE: f32 = 1000.0;
const MAX_ATTACK_SPEED: f32 = 20.0;
/// Widest area a sweeping tool (sickle) clears around the block hit.
const MAX_SWEEP: u8 = 4;

/// A material tier of tools (`tiers/<name>.ron`): wood, stone, iron, gold, diamond, and
/// later Ignium, Nihilium and Siderium, each one more level.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Tier", deny_unknown_fields)]
pub struct Tier {
    /// Mining level of its tools: 0 wood, 1 stone, 2 iron, 3 diamond, 4 and more beyond.
    pub level: u8,
    /// Breaking speed of its tools on their blocks (a bare hand is 1).
    pub speed: f32,
    /// Uses of its tools and weapons.
    pub durability: u16,
    /// Added to the base damage of its weapons.
    #[serde(default)]
    pub damage: f32,
    /// How well it takes enchantments, once enchanting exists.
    #[serde(default)]
    pub enchantability: u8,
    /// Upgrade slots of its tools and armour, one family each (`upgrade_slots/`):
    /// filled at the anvil. An item may list its own instead.
    #[serde(default)]
    pub upgrade_slots: Vec<Identifier>,
    /// Armour of this tier that gives no durability lasts this × 11, 16, 15 or 13 uses
    /// (head, chest, legs, feet).
    #[serde(default)]
    pub armor_durability: Option<u16>,
    /// Mends its items at the anvil (an ingot, planks).
    #[serde(default)]
    pub repair: Option<IngredientDef>,
}

impl Tier {
    pub(super) fn validated(
        mut self,
        families: &BTreeMap<Identifier, UpgradeSlotFamily>,
        source: &Path,
        errors: &mut Vec<PackError>,
    ) -> Option<Self> {
        if self.durability == 0 {
            errors.push(PackError::invalid(source, "a tier needs a durability"));
            return None;
        }
        self.upgrade_slots = valid_slots(self.upgrade_slots, families, source, errors);
        self.damage = bounded(self.damage, 0.0, MAX_DAMAGE);
        Some(self)
    }
}

/// `tool` of an item file: level and speed come from its tier when left out.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Tool", deny_unknown_fields)]
pub struct ToolSpec {
    /// Tool kinds it counts as (`strata:pickaxe`); several for a multi-tool.
    pub kinds: Vec<Identifier>,
    #[serde(default)]
    pub level: Option<u8>,
    #[serde(default)]
    pub speed: Option<f32>,
    /// Breaking an instant block of one of its kinds also breaks those of the same kind
    /// up to this many blocks away (a sickle through tall grass).
    #[serde(default)]
    pub sweep: u8,
}

impl ToolSpec {
    pub(super) fn resolve(
        self,
        tier: Option<&Tier>,
        source: &Path,
        errors: &mut Vec<PackError>,
    ) -> Option<ToolDef> {
        if self.kinds.is_empty() {
            errors.push(PackError::invalid(source, "a tool needs at least one kind"));
            return None;
        }
        if self.sweep > MAX_SWEEP {
            errors.push(PackError::invalid(
                source,
                format!("sweep: {MAX_SWEEP} at most"),
            ));
        }
        let tool = ToolDef {
            kinds: self.kinds,
            level: self.level.or(tier.map(|t| t.level)).unwrap_or(0),
            speed: self.speed.or(tier.map(|t| t.speed)).unwrap_or(1.0),
            sweep: self.sweep.min(MAX_SWEEP),
        };
        Some(tool.clamped())
    }
}

/// `weapon` of an item: damage of one hit (before the tier's is added) and hits per
/// second. Combat uses them from milestone 10.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename = "Weapon", deny_unknown_fields)]
pub struct WeaponDef {
    pub damage: f32,
    #[serde(default = "one")]
    pub attack_speed: f32,
}

fn one() -> f32 {
    1.0
}

impl WeaponDef {
    pub(super) fn with_tier(self, tier: Option<&Tier>) -> Self {
        let damage = self.damage + tier.map_or(0.0, |t| t.damage);
        Self {
            damage: bounded(damage, 0.0, MAX_DAMAGE),
            attack_speed: bounded(self.attack_speed, 0.1, MAX_ATTACK_SPEED),
        }
    }

    /// Damage per second when hitting as fast as allowed.
    pub fn per_second(&self) -> f32 {
        self.damage * self.attack_speed
    }
}

/// `value` within `min..=max`, `min` when not a number (a pack typo).
pub(super) fn bounded(value: f32, min: f32, max: f32) -> f32 {
    if value.is_nan() {
        return min;
    }
    value.clamp(min, max)
}
