//! Stacks as saved: by item identifier, so a world survives packs being added or
//! reordered. An item whose pack is gone leaves an empty slot.

use serde::{Deserialize, Serialize};
use strata_core::content::{Item, ItemId, Registry};
use strata_core::id::Identifier;
use strata_core::inventory::{Container, ItemData, ItemStack, PlayerInventory};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Stack")]
pub struct SavedStack {
    pub item: Identifier,
    pub count: u16,
    #[serde(default)]
    pub durability: Option<u16>,
    #[serde(default)]
    pub data: ItemData,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Inventory")]
pub struct SavedInventory {
    pub slots: Vec<Option<SavedStack>>,
    #[serde(default)]
    pub locked: Vec<bool>,
    #[serde(default)]
    pub selected: u8,
}

impl SavedStack {
    pub fn from_stack(stack: &ItemStack, items: &Registry<ItemId, Item>) -> Option<Self> {
        Some(Self {
            item: items.identifier(stack.item)?.clone(),
            count: stack.count,
            durability: stack.durability,
            data: stack.data.clone(),
        })
    }

    pub fn to_stack(&self, items: &Registry<ItemId, Item>) -> Option<ItemStack> {
        Some(ItemStack {
            item: items.key(&self.item)?,
            count: self.count,
            durability: self.durability,
            data: self.data.clone(),
        })
    }
}

pub fn save_slots(
    slots: &[Option<ItemStack>],
    items: &Registry<ItemId, Item>,
) -> Vec<Option<SavedStack>> {
    slots
        .iter()
        .map(|slot| slot.as_ref().and_then(|s| SavedStack::from_stack(s, items)))
        .collect()
}

pub fn load_slots(slots: &[Option<SavedStack>], items: &Registry<ItemId, Item>) -> Container {
    Container {
        slots: slots
            .iter()
            .map(|slot| slot.as_ref().and_then(|s| s.to_stack(items)))
            .collect(),
    }
}

impl SavedInventory {
    pub fn from_inventory(inventory: &PlayerInventory, items: &Registry<ItemId, Item>) -> Self {
        Self {
            slots: save_slots(&inventory.slots.slots, items),
            locked: inventory.locked.clone(),
            selected: inventory.selected,
        }
    }

    pub fn to_inventory(&self, items: &Registry<ItemId, Item>) -> PlayerInventory {
        let mut inventory = PlayerInventory {
            slots: load_slots(&self.slots, items),
            locked: self.locked.clone(),
            selected: self.selected,
            cursor: None,
        };
        inventory.normalize();
        inventory
    }
}
