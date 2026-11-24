//! Chests and barrels by tier (`storage_tiers/`), what their upgrades do (sorting, filter,
//! compression by `compression/` recipes, destruction, magnet), and how every block
//! entity lays out its slots: its own first, then its upgrade slots.

mod behaviour;
mod defs;
mod filter;
mod load;
mod slots;

pub use behaviour::{Upkeep, after_entry, compress, destroy_entered, sort_storage, take_in};
pub use defs::{CompressionDef, StorageTier, Upgradable};
pub use filter::{FILTER_CELLS, ItemFilter, filter_of, set_filter_cell, toggle_filter_mode};
pub use slots::StorageSlots;

pub(crate) use load::load_storage;

use crate::content::{BlockId, Content, ItemId};
use crate::cooking::FURNACE_MODULES;
use crate::id::Identifier;
use std::collections::BTreeMap;

/// Nine of `from` make one `to` in a storage with a compression upgrade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compression {
    pub from: ItemId,
    pub to: ItemId,
    pub count: u16,
}

/// Storage tiers and compression recipes of the active packs.
#[derive(Debug, Clone, Default)]
pub struct Storage {
    pub tiers: BTreeMap<Identifier, StorageTier>,
    /// In identifier order; nuggets before ingots when chained.
    pub compressions: Vec<Compression>,
}

/// The slots of a block entity: `storage` of its own (a chest's grid, a furnace's input,
/// fuel and output), then `upgrades` upgrade slots of the `family`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotLayout {
    pub storage: usize,
    pub upgrades: usize,
    pub family: Option<Identifier>,
}

impl SlotLayout {
    pub fn total(&self) -> usize {
        self.storage + self.upgrades
    }
}

impl Content {
    /// How the block entity of `block` lays out its slots, if it has one.
    pub fn slot_layout(&self, block: BlockId) -> Option<SlotLayout> {
        let def = self.blocks.get(block)?;
        if let Some(container) = &def.container {
            let tier = container
                .upgradable
                .as_ref()
                .and_then(|u| self.storage.tiers.get(&u.tier));
            return Some(SlotLayout {
                storage: container.slots(),
                upgrades: tier.map_or(0, |t| usize::from(t.upgrade_slots)),
                family: tier.map(|t| t.upgrade_family.clone()),
            });
        }
        let furnace = def.furnace.as_ref()?;
        let tier = self.cooking.tier(&furnace.tier);
        Some(SlotLayout {
            storage: FURNACE_MODULES,
            upgrades: tier.map_or(0, |t| usize::from(t.upgrade_slots)),
            family: tier.map(|t| t.upgrade_family.clone()),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
