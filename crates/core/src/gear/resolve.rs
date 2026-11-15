//! An item's equipment fields checked against the tiers, weights and slot families.

use super::slots::valid_slots;
use super::{ArmorDef, Gear, ItemGear, Tier, ToolSpec, WeaponDef, piece_durability};
use crate::content::{EquipSlot, ToolDef};
use crate::crafting::IngredientDef;
use crate::id::Identifier;
use crate::pack::PackError;
use std::path::Path;

/// The equipment fields of an item file, before they are checked.
#[derive(Default)]
pub struct GearFields {
    pub tier: Option<Identifier>,
    pub tool: Option<ToolSpec>,
    pub weapon: Option<WeaponDef>,
    pub armor: Option<ArmorDef>,
    pub durability: Option<u16>,
    pub equip: Option<EquipSlot>,
    /// The item's own slots, instead of its tier's.
    pub upgrade_slots: Option<Vec<Identifier>>,
    /// The item's own repair material, instead of its tier's.
    pub repair: Option<IngredientDef>,
    /// Bricks given while worn.
    pub worn: Vec<crate::upgrade::Effect>,
}

/// An item's equipment once resolved: tier values fill what the item leaves out.
pub struct Resolved {
    pub gear: ItemGear,
    pub tool: Option<ToolDef>,
    pub durability: Option<u16>,
}

impl Gear {
    /// Fills an item's tool, weapon, armour, durability and slots from its tier and weight.
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
        let wielded = tool.is_some() || weapon.is_some();
        let derived = derived_uses(tier, fields.equip, wielded, armor.is_some());
        let durability = fields.durability.or(derived).filter(|&uses| uses > 0);
        if armor.is_some() && durability.is_none() {
            errors.push(PackError::invalid(
                source,
                "armour needs a durability, or a tier with armor_durability and an equip slot",
            ));
        }
        let upgrade_slots = match fields.upgrade_slots {
            Some(own) => valid_slots(own, &self.slot_families, source, errors),
            None => tier.map_or_else(Vec::new, |tier| tier.upgrade_slots.clone()),
        };
        Resolved {
            gear: ItemGear {
                tier: fields.tier.filter(|_| tier.is_some()),
                weapon,
                armor,
                upgrade_slots,
                enchantability: tier.map_or(0, |tier| tier.enchantability),
                repair: fields
                    .repair
                    .or_else(|| tier.and_then(|tier| tier.repair.clone())),
                worn: if fields.equip.is_some() {
                    fields.worn
                } else {
                    Vec::new()
                },
            },
            tool,
            durability,
        }
    }
}

/// Uses given by the tier: its tool uses for tools and weapons, its armour factor times
/// the piece's share for armour.
fn derived_uses(
    tier: Option<&Tier>,
    equip: Option<EquipSlot>,
    wielded: bool,
    worn: bool,
) -> Option<u16> {
    let tier = tier?;
    if wielded {
        return Some(tier.durability);
    }
    let factor = tier.armor_durability.filter(|_| worn)?;
    Some(factor.saturating_mul(piece_durability(equip?)?))
}
