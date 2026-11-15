//! Item upgrades (`upgrades/`): bricks of effects put in an item's typed slots at the
//! anvil, kept in the stack's data, and the single computation of what an item does
//! (base, then tier, then upgrades). Crafted upgrades come first; machine modules, runes
//! and staff parts are other slot families and other upgrades, with the same engine.

mod anvil;
mod area;
mod cache;
mod defs;
mod describe;
mod installed;
mod load;
mod stats;

pub use anvil::{
    ANVIL_ITEM, ANVIL_MATERIAL, Blocked, Choice, Outcome, anvil_outcome, choices, take_outcome,
};
pub use area::{area_bounds, area_cells, fits_area, swing_cells, swing_ticks};
pub use cache::StatsCache;
pub use defs::{AnvilRules, AreaSize, Effect, ItemStatDef, LevelDef, StatOp, UpgradeDef};
pub use installed::{chosen_area, installed, repairs, with_area, with_upgrade};
pub use stats::{ItemStats, applies, max_durability, stack_stats, stack_tool};

use crate::content::ItemId;
use crate::crafting::Ingredient;
use crate::id::Identifier;
use std::collections::{BTreeMap, HashMap};

/// An upgrade, its materials resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Upgrade {
    pub id: Identifier,
    pub slot: Identifier,
    /// The items it suits; any item with a slot of its family when `None`.
    pub applies_to: Option<Ingredient>,
    pub levels: Vec<UpgradeLevel>,
    pub requires: Vec<Identifier>,
    pub excludes: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpgradeLevel {
    pub effects: Vec<Effect>,
    pub xp: u32,
    pub material: Ingredient,
    pub count: u16,
}

/// The upgrades of the active packs, in identifier order (a choice's index), and what
/// mends each item.
#[derive(Debug, Clone, Default)]
pub struct Upgrades {
    list: Vec<Upgrade>,
    pub repairs: HashMap<ItemId, Ingredient>,
    pub anvil: AnvilRules,
    /// Stats effects may change (`item_stats/`).
    pub stats: BTreeMap<Identifier, ItemStatDef>,
}

impl Upgrades {
    pub fn new(
        mut list: Vec<Upgrade>,
        repairs: HashMap<ItemId, Ingredient>,
        anvil: AnvilRules,
    ) -> Self {
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Self {
            list,
            repairs,
            anvil,
            stats: BTreeMap::new(),
        }
    }

    pub fn list(&self) -> &[Upgrade] {
        &self.list
    }

    pub fn get(&self, id: &Identifier) -> Option<&Upgrade> {
        self.list
            .binary_search_by(|upgrade| upgrade.id.cmp(id))
            .ok()
            .and_then(|index| self.list.get(index))
    }

    pub fn index_of(&self, id: &Identifier) -> Option<u16> {
        let index = self.list.binary_search_by(|u| u.id.cmp(id)).ok()?;
        u16::try_from(index).ok()
    }

    pub fn at(&self, index: u16) -> Option<&Upgrade> {
        self.list.get(usize::from(index))
    }
}

pub use load::MAX_AREA;
pub(crate) use load::load_upgrades;

#[cfg(test)]
mod tests;
