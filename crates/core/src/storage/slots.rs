//! The slots of an open chest or barrel with upgrade slots: its storage (both halves of
//! a double chest), then the upgrade slots of each half.

use super::filter::ItemFilter;
use crate::bag::{BagRules, Bags};
use crate::content::{Item, ItemId, Registry};
use crate::inventory::{Container, ItemStack, SlotPolicy};
use crate::upgrade::{SlotOwner, Upgrades};
use std::ops::Range;

pub struct StorageSlots<'a> {
    pub items: &'a Registry<ItemId, Item>,
    pub upgrades: &'a Upgrades,
    pub owner: SlotOwner<'a>,
    /// Slots of storage before the upgrade slots.
    pub storage: usize,
    /// What filter upgrades let in by shift click (everything without one).
    pub filters: Vec<ItemFilter>,
    /// A bag's storage takes no bag, and a pouch only its tag's items.
    pub bag: Option<(&'a Bags, &'a BagRules)>,
}

impl StorageSlots<'_> {
    fn bag_admits(&self, stack: &ItemStack) -> bool {
        self.bag
            .is_none_or(|(bags, rules)| bags.admits(rules, stack))
    }

    fn is_upgrade(&self, stack: &ItemStack) -> bool {
        self.upgrades.of_item(self.items, stack.item).is_some()
    }

    fn upgrade_fits(&self, slots: &Container, index: usize, stack: &ItemStack) -> bool {
        let others = (self.storage..slots.len())
            .filter(|&other| other != index)
            .filter_map(|other| slots.get(other));
        (self.upgrades).fits_slot(self.items, self.owner, others, stack)
    }
}

impl SlotPolicy for StorageSlots<'_> {
    /// By hand anything goes in the storage (but a bag's takes no bag); an upgrade slot takes an upgrade that suits.
    fn accepts(&self, slots: &Container, index: usize, stack: &ItemStack) -> bool {
        if index < self.storage {
            return self.bag_admits(stack);
        }
        self.upgrade_fits(slots, index, stack)
    }

    /// An upgrade goes to a free upgrade slot it fits; anything else to the storage, if
    /// the filter lets it in.
    fn route(&self, slots: &Container, stack: &ItemStack) -> Vec<usize> {
        if self.is_upgrade(stack) {
            let free = (self.storage..slots.len())
                .find(|&i| slots.get(i).is_none() && self.upgrade_fits(slots, i, stack));
            if let Some(free) = free {
                return vec![free];
            }
        }
        if !self.lets_in(stack) {
            return Vec::new();
        }
        (0..self.storage.min(slots.len())).collect()
    }

    fn storage(&self) -> Option<Range<usize>> {
        Some(0..self.storage)
    }

    fn lets_in(&self, stack: &ItemStack) -> bool {
        self.bag_admits(stack) && self.filters.iter().all(|f| f.lets_in(stack.item))
    }
}
