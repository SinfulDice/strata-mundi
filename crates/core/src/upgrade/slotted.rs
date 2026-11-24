//! Upgrades that are items put in a block's upgrade slots (furnace modules, storage
//! upgrades; the backpack's next): which one fits where, and the bricks they give
//! together, counted like an item's (the same brick once, the strongest).

use super::defs::Effect;
use super::stats::strongest_bricks;
use super::{Upgrade, Upgrades};
use crate::content::{Item, ItemId, Registry};
use crate::id::Identifier;
use crate::inventory::ItemStack;

/// Whose upgrade slots these are: their family (`upgrade_slots/`) and the item of the
/// block they belong to (what `applies_to` tags name).
#[derive(Debug, Clone, Copy)]
pub struct SlotOwner<'a> {
    /// Family of its upgrade slots (none: it has none).
    pub family: Option<&'a Identifier>,
    pub item: Option<ItemId>,
}

impl Upgrades {
    /// The upgrade an item is in a block's slot.
    pub fn of_item(&self, items: &Registry<ItemId, Item>, item: ItemId) -> Option<&Upgrade> {
        self.get(items.get(item)?.upgrade.as_ref()?)
    }

    /// Whether `stack` may go in an upgrade slot of `owner` beside the upgrades in
    /// `others`: of the slots' family, suiting the block, not there already, needing
    /// none missing and excluding none present.
    pub fn fits_slot<'s>(
        &self,
        items: &Registry<ItemId, Item>,
        owner: SlotOwner<'_>,
        others: impl Iterator<Item = &'s ItemStack>,
        stack: &ItemStack,
    ) -> bool {
        let Some(upgrade) = self.of_item(items, stack.item) else {
            return false;
        };
        let suits = upgrade
            .applies_to
            .as_ref()
            .is_none_or(|targets| owner.item.is_some_and(|item| targets.accepts(item)));
        if owner.family != Some(&upgrade.slot) || !suits {
            return false;
        }
        let present: Vec<&Upgrade> = others
            .filter_map(|other| self.of_item(items, other.item))
            .collect();
        let clash = present.iter().any(|other| {
            other.id == upgrade.id
                || other.excludes.contains(&upgrade.id)
                || upgrade.excludes.contains(&other.id)
        });
        let missing = upgrade
            .requires
            .iter()
            .any(|needed| !present.iter().any(|other| other.id == *needed));
        !clash && !missing
    }

    /// What the upgrades in these slots do together: level one of each, the same upgrade
    /// once, the strongest gain of each stat, every cost, additions first.
    pub fn slotted_effects<'s>(
        &self,
        items: &Registry<ItemId, Item>,
        slots: impl Iterator<Item = &'s ItemStack>,
    ) -> Vec<Effect> {
        let mut held: Vec<&Upgrade> = slots
            .filter_map(|stack| self.of_item(items, stack.item))
            .collect();
        held.sort_by(|a, b| a.id.cmp(&b.id));
        held.dedup_by(|a, b| a.id == b.id);
        let effects = held
            .into_iter()
            .filter_map(|upgrade| upgrade.levels.first())
            .flat_map(|level| level.effects.iter().cloned());
        strongest_bricks(effects)
    }
}
