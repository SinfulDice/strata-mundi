//! Tools, weapons and armour beyond mining: material tiers (`tiers/`), armour weights
//! (`armor_weights/`), upgrade slot families (`upgrade_slots/`), the weapon and armour
//! stats of items, and what worn armour adds up to (protection, burden).

mod armor;
mod resolve;
mod slots;
mod tier;

pub use armor::{
    Armor, ArmorDef, ArmorWeight, Burden, MAX_BURDEN, piece_durability, worn_burden,
    worn_protection,
};
pub use resolve::{GearFields, Resolved};
pub use slots::{MAX_UPGRADE_SLOTS, UpgradeSlotFamily, count_slots};
pub use tier::{Tier, ToolSpec, WeaponDef};

use crate::crafting::IngredientDef;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use std::collections::BTreeMap;

/// Tiers, armour weights and upgrade slot families of the active packs.
#[derive(Debug, Clone, Default)]
pub struct Gear {
    pub tiers: BTreeMap<Identifier, Tier>,
    pub weights: BTreeMap<Identifier, ArmorWeight>,
    pub slot_families: BTreeMap<Identifier, UpgradeSlotFamily>,
}

/// What an item is as equipment, its tool and durability aside.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemGear {
    pub tier: Option<Identifier>,
    /// Damage already includes the tier's.
    pub weapon: Option<WeaponDef>,
    pub armor: Option<Armor>,
    /// One family per slot (the item's own list, or its tier's): the anvil fills them.
    pub upgrade_slots: Vec<Identifier>,
    /// From the tier; read by enchanting when it comes.
    pub enchantability: u8,
    /// Mends it at the anvil: the item's own, or its tier's.
    pub repair: Option<IngredientDef>,
}

impl Gear {
    pub fn load(packs: &PackSet, errors: &mut Vec<PackError>) -> Self {
        let slot_families: BTreeMap<_, _> = packs
            .layered::<UpgradeSlotFamily>("upgrade_slots", errors)
            .into_iter()
            .map(|(id, Sourced { value, .. })| (id, value))
            .collect();
        let tiers = packs
            .layered::<Tier>("tiers", errors)
            .into_iter()
            .filter_map(|(id, Sourced { value, source })| {
                let tier = value.validated(&slot_families, &source, errors)?;
                Some((id, tier))
            })
            .collect();
        let weights = packs
            .layered::<ArmorWeight>("armor_weights", errors)
            .into_iter()
            .map(|(id, Sourced { value, .. })| (id, value.clamped()))
            .collect();
        Self {
            tiers,
            weights,
            slot_families,
        }
    }
}

#[cfg(test)]
mod tests;
