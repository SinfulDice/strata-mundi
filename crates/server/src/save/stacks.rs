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
    /// Carried under the pointer when saved (an autosave with a screen open); given
    /// back on loading.
    #[serde(default)]
    pub cursor: Option<SavedStack>,
    /// In the crafting grid when saved (a screen open); given back on loading.
    #[serde(default)]
    pub craft: Vec<SavedStack>,
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
            cursor: inventory
                .cursor
                .as_ref()
                .and_then(|stack| SavedStack::from_stack(stack, items)),
            craft: Vec::new(),
        }
    }

    /// The crafting grid's stacks, saved with the inventory.
    pub fn with_grid(mut self, grid: &[ItemStack], items: &Registry<ItemId, Item>) -> Self {
        self.craft = grid
            .iter()
            .filter_map(|stack| SavedStack::from_stack(stack, items))
            .collect();
        self
    }

    pub fn grid(&self, items: &Registry<ItemId, Item>) -> Vec<ItemStack> {
        self.craft
            .iter()
            .filter_map(|s| s.to_stack(items))
            .collect()
    }

    pub fn to_inventory(&self, items: &Registry<ItemId, Item>) -> PlayerInventory {
        let mut inventory = PlayerInventory {
            slots: load_slots(&self.slots, items),
            locked: self.locked.clone(),
            selected: self.selected,
            cursor: self.cursor.as_ref().and_then(|stack| stack.to_stack(items)),
        };
        inventory.normalize();
        inventory
    }
}
