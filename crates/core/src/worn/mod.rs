//! What the player wears adds up here: armour, accessories (rings, amulet, belt, cloak)
//! and set bonuses (`sets/`), with the movement abilities equipment gives
//! (`mobility.ron`). One computation for the server and the client's prediction.

mod cache;
mod describe;
mod load;
mod mobility;
mod sets;
mod stats;

pub use cache::{Equipment, WornCache};
pub use mobility::MobilityRules;
pub use sets::{BonusDef, GearSet, SetBonus, SetDef};
pub use stats::{SetProgress, WornStat, WornStats, inventory_worn_stats, worn_stats};

pub(crate) use load::{bound_worn_effects, load_worn};

/// What identifies the worn slots for `WornCache`: item, data and broken, slot by slot.
fn inventory_slots_key(
    inventory: &crate::inventory::PlayerInventory,
) -> impl Iterator<Item = Option<(crate::content::ItemId, &crate::inventory::ItemData, bool)>> + '_
{
    crate::inventory::worn_slots().map(|slot| {
        let stack = inventory.slots.get(slot)?;
        Some((stack.item, &stack.data, stack.is_broken()))
    })
}

/// The sets and the mobility rules of the active packs.
#[derive(Debug, Clone, Default)]
pub struct Worn {
    /// In identifier order.
    pub sets: Vec<GearSet>,
    pub mobility: MobilityRules,
}

impl Worn {
    /// The sets `item` belongs to.
    pub fn sets_of(&self, item: crate::content::ItemId) -> impl Iterator<Item = &GearSet> {
        self.sets
            .iter()
            .filter(move |set| set.pieces.contains(&item))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
