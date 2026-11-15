use super::tier::bounded;
use crate::content::{EquipSlot, Item, ItemId, Registry};
use crate::id::Identifier;
use crate::inventory::ItemStack;
use crate::pack::PackError;
use crate::upgrade::{Upgrades, stack_stats};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Worn armour never slows, tires or drains more than this share.
pub const MAX_BURDEN: f32 = 0.9;
const MAX_PROTECTION: u16 = 1000;

/// What one worn piece of a weight costs (`armor_weights/<name>.ron`): light, medium,
/// heavy. Shares of walking speed, stamina and mana; four heavy pieces add up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename = "ArmorWeight", deny_unknown_fields)]
pub struct ArmorWeight {
    #[serde(default)]
    pub speed: f32,
    /// Read by the stamina gauge (milestone 8).
    #[serde(default)]
    pub stamina: f32,
    /// Read by magic (v0.9).
    #[serde(default)]
    pub mana: f32,
}

impl ArmorWeight {
    pub(super) fn clamped(self) -> Self {
        let share = |value| bounded(value, 0.0, MAX_BURDEN);
        Self {
            speed: share(self.speed),
            stamina: share(self.stamina),
            mana: share(self.mana),
        }
    }
}

/// Share of one piece in its set's uses (Minecraft's): multiplied by the tier's
/// `armor_durability`.
pub fn piece_durability(slot: EquipSlot) -> u16 {
    match slot {
        EquipSlot::Head => 11,
        EquipSlot::Chest => 16,
        EquipSlot::Legs => 15,
        EquipSlot::Feet => 13,
    }
}

/// `armor` of an item file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Armor", deny_unknown_fields)]
pub struct ArmorDef {
    pub protection: u16,
    /// An `armor_weights` entry.
    pub weight: Identifier,
}

/// A piece of armour: its protection, its weight and what that weight costs.
#[derive(Debug, Clone, PartialEq)]
pub struct Armor {
    pub protection: u16,
    pub weight: Identifier,
    pub burden: Burden,
}

impl ArmorDef {
    pub(super) fn resolve(
        self,
        weights: &BTreeMap<Identifier, ArmorWeight>,
        source: &Path,
        errors: &mut Vec<PackError>,
    ) -> Option<Armor> {
        let Some(weight) = weights.get(&self.weight) else {
            errors.push(PackError::invalid(
                source,
                format!("unknown armour weight `{}`", self.weight),
            ));
            return None;
        };
        Some(Armor {
            protection: self.protection.min(MAX_PROTECTION),
            burden: Burden::from(*weight),
            weight: self.weight,
        })
    }
}

/// Shares of speed, stamina and mana taken by what is worn.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Burden {
    pub speed: f32,
    pub stamina: f32,
    pub mana: f32,
}

/// Both burdens together, within `MAX_BURDEN`.
impl std::ops::Add for Burden {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            speed: (self.speed + other.speed).min(MAX_BURDEN),
            stamina: (self.stamina + other.stamina).min(MAX_BURDEN),
            mana: (self.mana + other.mana).min(MAX_BURDEN),
        }
    }
}

impl From<ArmorWeight> for Burden {
    fn from(weight: ArmorWeight) -> Self {
        Self {
            speed: weight.speed,
            stamina: weight.stamina,
            mana: weight.mana,
        }
    }
}

impl Burden {
    /// Multiplies walking speed: 1 unburdened.
    pub fn speed_factor(&self) -> f32 {
        1.0 - self.speed.clamp(0.0, MAX_BURDEN)
    }
}

fn armor_of(
    stack: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Option<Armor> {
    stack_stats(stack, items, upgrades).armor
}

/// Protection points of the worn pieces, upgrades counted; a broken one (kept until
/// repaired) protects nothing.
pub fn worn_protection<'a>(
    worn: impl IntoIterator<Item = &'a ItemStack>,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> u16 {
    worn.into_iter()
        .filter(|stack| !stack.is_broken())
        .filter_map(|stack| armor_of(stack, items, upgrades))
        .fold(0, |total, armor| total.saturating_add(armor.protection))
}

/// What the worn pieces weigh, broken or not, lightening counted.
pub fn worn_burden<'a>(
    worn: impl IntoIterator<Item = &'a ItemStack>,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Burden {
    worn.into_iter()
        .filter_map(|stack| armor_of(stack, items, upgrades))
        .fold(Burden::default(), |total, armor| total + armor.burden)
}
