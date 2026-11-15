//! Stats of the slots looked at every tick or frame (the hand, the armour), worked out
//! again only when the stack's item or upgrades change: wear does not change them.

use super::{ItemStats, Upgrades, stack_stats};
use crate::content::{Item, ItemId, Registry};
use crate::inventory::{ItemData, ItemStack};
use std::cell::RefCell;
use std::sync::Arc;

type Entry = (usize, ItemId, ItemData, Arc<ItemStats>);

#[derive(Debug, Clone, Default)]
pub struct StatsCache {
    entries: RefCell<Vec<Entry>>,
}

impl StatsCache {
    /// The stats of `stack`, lying in `slot`.
    pub fn get(
        &self,
        slot: usize,
        stack: &ItemStack,
        items: &Registry<ItemId, Item>,
        upgrades: &Upgrades,
    ) -> Arc<ItemStats> {
        let mut entries = self.entries.borrow_mut();
        let found = entries
            .iter()
            .find(|(at, item, data, _)| *at == slot && *item == stack.item && *data == stack.data);
        if let Some((.., stats)) = found {
            return Arc::clone(stats);
        }
        let stats = Arc::new(stack_stats(stack, items, upgrades));
        entries.retain(|(at, ..)| *at != slot);
        entries.push((slot, stack.item, stack.data.clone(), Arc::clone(&stats)));
        stats
    }

    /// New packs: everything is worked out again.
    pub fn clear(&self) {
        self.entries.borrow_mut().clear();
    }
}

/// A cache holds no state of its own: two are always alike.
impl PartialEq for StatsCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
