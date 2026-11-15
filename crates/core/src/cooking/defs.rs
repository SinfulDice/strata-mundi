//! What the packs write about furnaces, fuels, cooking and stone cutting.

use crate::crafting::{IngredientDef, ResultDef};
use crate::id::Identifier;
use serde::Deserialize;

/// `furnace` of a block (furnace, smoker, blast furnace, of any tier).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "Furnace", deny_unknown_fields)]
pub struct FurnaceDef {
    /// Blocks of one family share kits: a kit turns a block into the block of the same
    /// family with the next tier.
    pub family: Identifier,
    /// A file of `furnace_tiers/`.
    pub tier: Identifier,
    /// Kinds of recipes it cooks; empty: every kind.
    #[serde(default)]
    pub kinds: Vec<Identifier>,
    /// Percent of the normal speed (fuel burns as much faster): 200 for the smoker and
    /// the blast furnace.
    #[serde(default = "hundred")]
    pub speed: u16,
}

/// `furnace_tiers/<name>.ron`: how much better a tier is than the stone furnace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename = "FurnaceTier", deny_unknown_fields)]
pub struct FurnaceTier {
    /// Percent of a recipe's time one cooking takes.
    #[serde(default = "hundred")]
    pub cook_time: u16,
    /// Percent of the cookings one fuel gives.
    #[serde(default = "hundred")]
    pub fuel_value: u16,
    /// Slots for modules.
    #[serde(default)]
    pub upgrade_slots: u8,
}

impl Default for FurnaceTier {
    fn default() -> Self {
        Self {
            cook_time: 100,
            fuel_value: 100,
            upgrade_slots: 0,
        }
    }
}

/// `cooking/<name>.ron`: one item cooked into another.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Cooking", deny_unknown_fields)]
pub struct CookingDef {
    pub ingredient: IngredientDef,
    pub result: ResultDef,
    /// Ticks in a stone furnace (200 = 10 s).
    #[serde(default = "default_time")]
    pub time: u32,
    /// Experience stored in the furnace per cooking (given out from v0.3 milestone 9).
    #[serde(default)]
    pub experience: f32,
    /// Which furnaces cook it (`strata:food`, `strata:ore`, `strata:misc`).
    pub kind: Identifier,
}

/// `stonecutting/<name>.ron`: one block cut into a variant, with no fuel.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Cutting", deny_unknown_fields)]
pub struct CuttingDef {
    pub ingredient: IngredientDef,
    pub result: ResultDef,
}

/// `fuels/<name>.ron`: how long items burn in a stone furnace.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Fuel", deny_unknown_fields)]
pub struct FuelDef {
    pub ingredient: IngredientDef,
    /// Ticks (coal: 1 600, eight cookings).
    pub burn_time: u32,
    /// The fuel screens and the wiki count cookings in (« 8 cookings per coal »); the
    /// first one marked wins.
    #[serde(default)]
    pub reference: bool,
}

/// `kit` of an item: used on a block of tier `from`, makes it the block of the same
/// family of tier `to`, keeping what it holds.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "Kit", deny_unknown_fields)]
pub struct KitDef {
    pub from: Identifier,
    pub to: Identifier,
}

/// `module` of an item: what it changes once in an upgrade slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum ModuleDef {
    /// In a furnace: percents of the time of a cooking and of the cookings per fuel, and
    /// how many results each cooking gives (double output: 2).
    Furnace {
        #[serde(default = "hundred")]
        cook_time: u16,
        #[serde(default = "hundred")]
        fuel_value: u16,
        #[serde(default = "one")]
        output: u8,
    },
}

fn hundred() -> u16 {
    100
}

fn one() -> u8 {
    1
}

/// Ticks of a cooking in a stone furnace when the recipe says nothing (10 s): also the
/// « usual cooking » screens and the wiki give times for.
pub const DEFAULT_COOK_TIME: u32 = 200;

fn default_time() -> u32 {
    DEFAULT_COOK_TIME
}
