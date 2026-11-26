//! Survival gauges: health (any body), the player's needs (food, thirst, stamina,
//! breath), kinds of damage, status effects, and what eating, drinking or touching a
//! block does. Everything is read from the packs; the server runs it, the client shows it.

mod defs;
mod describe;
mod effects;
mod harm;
mod health;
mod load;
mod needs;
mod rules;

pub use defs::{
    ConsumeDef, ContactDef, DamageType, EffectGrant, FillDef, Periodic, PeriodicDamage,
    StatusEffectDef,
};
pub use effects::{ActiveEffect, EffectPulse, StatusEffects};
pub use harm::{Defences, resistance, take_hit};
pub use health::{Health, fall_damage, reduced};
pub use needs::{Exertion, NeedHarm, Needs, NeedsContext, SurvivalOptions, breath_ticks};
pub use rules::SurvivalRules;

pub(crate) use load::{Known, load_survival};

use crate::content::{BlockId, ItemId};
use crate::id::Identifier;
use crate::inventory::{DataValue, ItemStack};
use std::collections::BTreeMap;

/// A kind of damage or an effect the engine deals or gives itself (`strata:fall`).
pub fn engine(name: &'static str) -> Identifier {
    Identifier::builtin("strata", name)
}

/// What the packs define about survival.
#[derive(Debug, Clone, Default)]
pub struct Survival {
    pub rules: SurvivalRules,
    pub damage: BTreeMap<Identifier, DamageType>,
    pub effects: BTreeMap<Identifier, StatusEffectDef>,
    /// What eating, drinking or applying each item does.
    pub consumables: BTreeMap<ItemId, Consumable>,
    /// Empty containers filled at a liquid.
    pub fills: BTreeMap<ItemId, Fill>,
    /// What touching each block does (by `BlockId`).
    pub contacts: Vec<Option<Contact>>,
    /// The liquid source one may drink from (`DrinkingRules::source`).
    pub drink_source: Option<BlockId>,
}

/// `ConsumeDef` with its items found.
#[derive(Debug, Clone, PartialEq)]
pub struct Consumable {
    pub ticks: u16,
    pub food: f32,
    pub saturation: f32,
    pub thirst: f32,
    pub effects: Vec<EffectGrant>,
    pub cures: Vec<Identifier>,
    pub leaves: Option<ItemId>,
    pub sips: Option<u8>,
    pub sound: Option<Identifier>,
    pub always: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub liquid: BlockId,
    pub fresh: ItemId,
    pub salty: ItemId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub damage: Identifier,
    pub amount: f32,
    pub every: u16,
    pub effect: Option<EffectGrant>,
}

/// Key of the sips left in a waterskin.
pub fn sips_key() -> Identifier {
    engine("sips")
}

/// Sips left in `stack` (a full container has them all).
pub fn sips_left(stack: &ItemStack, consumable: &Consumable) -> u8 {
    let full = consumable.sips.unwrap_or(1);
    match stack.data.get(&sips_key()) {
        Some(DataValue::Int(left)) => u8::try_from((*left).clamp(1, i64::from(full))).unwrap_or(1),
        _ => full,
    }
}

impl Survival {
    pub fn contact(&self, block: BlockId) -> Option<&Contact> {
        self.contacts.get(usize::from(block.0))?.as_ref()
    }

    /// Whether `item` does something when used in hand (eaten, drunk, applied).
    pub fn is_consumable(&self, item: ItemId) -> bool {
        self.consumables.contains_key(&item)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
