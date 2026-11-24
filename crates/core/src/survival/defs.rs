//! What the packs write about harm and care: kinds of damage (`damage_types/`), status
//! effects (`status_effects/`), what eating or using an item does (`consume` of an item),
//! and what touching a block does (`contact` of a block).

use crate::id::Identifier;
use serde::Deserialize;

/// `damage_types/<name>.ron`: a kind of harm (a fall, the fire, bleeding…).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "DamageType", deny_unknown_fields)]
pub struct DamageType {
    /// Armour protection lessens it (blows, falls); hunger or bleeding go through.
    #[serde(default)]
    pub armor: bool,
    /// The stat (`item_stats/`, in percent) that lessens it, from equipment, potions or
    /// spells (`strata:fire_resistance`).
    #[serde(default)]
    pub resistance: Option<Identifier>,
    /// It hits again during the invulnerability after a hit (effects over time, hunger).
    #[serde(default)]
    pub ignores_invulnerability: bool,
    /// The camera shakes and the screen flashes (not for slow harm like hunger).
    #[serde(default = "yes")]
    pub shakes: bool,
}

fn yes() -> bool {
    true
}

/// `status_effects/<name>.ron`: something lasting on a body (bleeding, healing, burning).
/// One effect is never there twice: given again, the strongest stays.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "StatusEffect", deny_unknown_fields)]
pub struct StatusEffectDef {
    /// Shown in red, cured by care items that name it.
    #[serde(default)]
    pub harmful: bool,
    /// Health given every `every` ticks, times the strength.
    #[serde(default)]
    pub heal: Option<Periodic>,
    /// Damage of `kind` dealt every `every` ticks, times the strength.
    #[serde(default)]
    pub damage: Option<PeriodicDamage>,
    /// Ends in water (burning).
    #[serde(default)]
    pub ends_in_water: bool,
    /// Stats it gives while it lasts (`item_stats/`, added, times the strength), like
    /// equipment: a resistance potion gives `strata:fire_resistance`.
    #[serde(default)]
    pub stats: Vec<(Identifier, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Periodic {
    pub amount: f32,
    pub every: u16,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeriodicDamage {
    pub kind: Identifier,
    pub amount: f32,
    pub every: u16,
}

/// An effect given by an item, a block or (later) a blow.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectGrant {
    pub effect: Identifier,
    pub seconds: f32,
    #[serde(default = "one")]
    pub strength: f32,
}

fn one() -> f32 {
    1.0
}

/// `consume` of an item: held down with the use button, it is eaten, drunk or applied.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Consume", deny_unknown_fields)]
pub struct ConsumeDef {
    /// Ticks the button is held (32 = 1.6 s, Minecraft's bite).
    #[serde(default = "bite")]
    pub ticks: u16,
    #[serde(default)]
    pub food: f32,
    #[serde(default)]
    pub saturation: f32,
    /// Thirst quenched (salt water makes it worse: negative).
    #[serde(default)]
    pub thirst: f32,
    #[serde(default)]
    pub effects: Vec<EffectGrant>,
    /// Effects it ends (a bandage stops bleeding).
    #[serde(default)]
    pub cures: Vec<Identifier>,
    /// What is left in hand (a bowl, an empty bottle).
    #[serde(default)]
    pub leaves: Option<Identifier>,
    /// Uses before it is empty (a waterskin), counted in the stack's data.
    #[serde(default)]
    pub sips: Option<u8>,
    /// Played while using it (`strata:eat`, `strata:drink`).
    #[serde(default)]
    pub sound: Option<Identifier>,
    /// May be used even when nothing is missing (care items, salt water).
    #[serde(default)]
    pub always: bool,
}

fn bite() -> u16 {
    32
}

/// `contact` of a block: touching it hurts (lava, fire) and may leave an effect.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Contact", deny_unknown_fields)]
pub struct ContactDef {
    pub damage: Identifier,
    pub amount: f32,
    #[serde(default)]
    pub effect: Option<EffectGrant>,
}

/// `fill` of an item: used on a source of `liquid`, it becomes `fresh`, or `salty` in a
/// biome of salt water (an empty bottle, an empty waterskin).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Fill", deny_unknown_fields)]
pub struct FillDef {
    pub liquid: Identifier,
    pub fresh: Identifier,
    pub salty: Identifier,
}
