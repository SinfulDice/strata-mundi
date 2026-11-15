//! Tools, weapons and armour beyond mining: material tiers (`tiers/`), armour weights
//! (`armor_weights/`), the weapon and armour stats of items, and what worn armour adds up
//! to (protection, burden).

mod armor;
mod tier;

pub use armor::{Armor, ArmorDef, ArmorWeight, Burden, MAX_BURDEN, worn_burden, worn_protection};
pub use tier::{MAX_UPGRADE_SLOTS, Tier, ToolSpec, WeaponDef};

use crate::content::ToolDef;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use std::collections::BTreeMap;
use std::path::Path;

/// Tiers and armour weights of the active packs.
#[derive(Debug, Clone, Default)]
pub struct Gear {
    pub tiers: BTreeMap<Identifier, Tier>,
    pub weights: BTreeMap<Identifier, ArmorWeight>,
}

/// What an item is as equipment, its tool and durability aside.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemGear {
    pub tier: Option<Identifier>,
    /// Damage already includes the tier's.
    pub weapon: Option<WeaponDef>,
    pub armor: Option<Armor>,
    /// Upgrade slots (from the tier): the anvil fills them.
    pub upgrade_slots: u8,
    /// From the tier; read by enchanting when it comes.
    pub enchantability: u8,
}

/// The equipment fields of an item file, before they are checked against the tiers.
pub struct GearFields {
    pub tier: Option<Identifier>,
    pub tool: Option<ToolSpec>,
    pub weapon: Option<WeaponDef>,
    pub armor: Option<ArmorDef>,
    pub durability: Option<u16>,
}

/// An item's equipment once resolved: tier values fill what the item leaves out.
pub struct Resolved {
    pub gear: ItemGear,
    pub tool: Option<ToolDef>,
    pub durability: Option<u16>,
}

impl Gear {
    pub fn load(packs: &PackSet, errors: &mut Vec<PackError>) -> Self {
        let tiers = packs
            .layered::<Tier>("tiers", errors)
            .into_iter()
            .filter_map(|(id, Sourced { value, source })| {
                value.validated(&source, errors).map(|tier| (id, tier))
            })
            .collect();
        let weights = packs
            .layered::<ArmorWeight>("armor_weights", errors)
            .into_iter()
            .map(|(id, Sourced { value, .. })| (id, value.clamped()))
            .collect();
        Self { tiers, weights }
    }

    /// Fills an item's tool, weapon, armour and durability from its tier and weight.
    pub fn resolve(
        &self,
        fields: GearFields,
        source: &Path,
        errors: &mut Vec<PackError>,
    ) -> Resolved {
        let tier = fields.tier.as_ref().and_then(|id| {
            let tier = self.tiers.get(id);
            if tier.is_none() {
                errors.push(PackError::invalid(source, format!("unknown tier `{id}`")));
            }
            tier
        });
        let tool = fields
            .tool
            .and_then(|spec| spec.resolve(tier, source, errors));
        let weapon = fields.weapon.map(|weapon| weapon.with_tier(tier));
        let armor = fields
            .armor
            .and_then(|armor| armor.resolve(&self.weights, source, errors));
        let tier_uses = tier
            .filter(|_| tool.is_some() || weapon.is_some())
            .map(|tier| tier.durability);
        Resolved {
            gear: ItemGear {
                tier: fields.tier.filter(|_| tier.is_some()),
                weapon,
                armor,
                upgrade_slots: tier.map_or(0, |tier| tier.upgrade_slots),
                enchantability: tier.map_or(0, |tier| tier.enchantability),
            },
            tool,
            durability: fields.durability.or(tier_uses).filter(|&uses| uses > 0),
        }
    }
}

#[cfg(test)]
mod tests;
