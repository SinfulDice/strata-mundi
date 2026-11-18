//! What is worn, worked out again only when a worn slot changes (its item, data or
//! being broken) or the rules do: the server asks every tick, the client every frame.

use super::{Worn, WornStats, inventory_slots_key};
use crate::content::ItemId;
use crate::inventory::{ItemData, PlayerInventory};
use crate::physics::Mobility;
use std::cell::RefCell;
use std::sync::Arc;

/// The worn stats and the abilities they give.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Equipment {
    pub stats: WornStats,
    pub mobility: Mobility,
}

pub(super) type SlotKey = Option<(ItemId, ItemData, bool)>;

#[derive(Debug, Clone, Default)]
pub struct WornCache {
    entry: RefCell<Option<(Vec<SlotKey>, Arc<Equipment>)>>,
}

impl WornCache {
    /// The equipment of `inventory`, from `compute` when a worn slot changed.
    pub fn get(
        &self,
        inventory: &PlayerInventory,
        compute: impl FnOnce() -> WornStats,
        rules: &Worn,
    ) -> Arc<Equipment> {
        let mut entry = self.entry.borrow_mut();
        if let Some((key, equipment)) = entry.as_ref()
            && inventory_slots_key(inventory)
                .eq(key.iter().map(|k| k.as_ref().map(|(i, d, b)| (*i, d, *b))))
        {
            return Arc::clone(equipment);
        }
        let stats = compute();
        let mobility = rules.mobility.mobility(&stats);
        let equipment = Arc::new(Equipment { stats, mobility });
        let key = inventory_slots_key(inventory)
            .map(|k| k.map(|(item, data, broken)| (item, data.clone(), broken)))
            .collect();
        *entry = Some((key, Arc::clone(&equipment)));
        equipment
    }

    /// New packs: worked out again.
    pub fn clear(&self) {
        self.entry.borrow_mut().take();
    }
}

/// A cache holds no state of its own: two are always alike.
impl PartialEq for WornCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
