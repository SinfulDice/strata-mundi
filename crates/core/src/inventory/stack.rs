use crate::content::{EquipSlot, Item, ItemId, Registry};
use crate::id::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Some items of one kind in one slot. Two stacks merge only when everything but the
/// count is equal (same wear, same upgrades).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item: ItemId,
    pub count: u16,
    /// Uses left, for items that wear out.
    #[serde(default)]
    pub durability: Option<u16>,
    #[serde(default)]
    pub data: ItemData,
}

/// Extensible facts about one stack (upgrades, handle parts, enchantments…), keyed by
/// namespaced identifiers so that packs can add their own without touching the engine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemData(pub BTreeMap<Identifier, DataValue>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataValue {
    Int(i64),
    Text(String),
    Id(Identifier),
    List(Vec<DataValue>),
}

impl ItemData {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: &Identifier) -> Option<&DataValue> {
        self.0.get(key)
    }

    pub fn set(&mut self, key: Identifier, value: DataValue) {
        self.0.insert(key, value);
    }
}

/// What the inventory needs to know about items.
pub trait ItemRules {
    /// Largest count of one stack (at least 1); 0 for an unknown item.
    fn max_stack(&self, item: ItemId) -> u16;
    /// Uses of a new one, for items that wear out.
    fn durability(&self, item: ItemId) -> Option<u16>;
    fn equip(&self, item: ItemId) -> Option<EquipSlot>;
}

impl ItemRules for Registry<ItemId, Item> {
    fn max_stack(&self, item: ItemId) -> u16 {
        self.get(item).map_or(0, |item| item.max_stack.max(1))
    }

    fn durability(&self, item: ItemId) -> Option<u16> {
        self.get(item)?.durability
    }

    fn equip(&self, item: ItemId) -> Option<EquipSlot> {
        self.get(item)?.equip
    }
}

impl ItemStack {
    /// `count` new items (unworn, without data).
    pub fn new(item: ItemId, count: u16, rules: &impl ItemRules) -> Self {
        Self {
            item,
            count,
            durability: rules.durability(item),
            data: ItemData::default(),
        }
    }

    /// Whether `other` could join this stack (all but the count equal).
    pub fn stacks_with(&self, other: &Self) -> bool {
        self.item == other.item && self.durability == other.durability && self.data == other.data
    }

    /// A stack is valid when it holds 1 to `max_stack` items.
    pub fn is_valid(&self, rules: &impl ItemRules) -> bool {
        (1..=rules.max_stack(self.item)).contains(&self.count)
    }

    /// Moves as many items of `other` as fit into this stack; returns how many moved.
    pub fn absorb(&mut self, other: &mut Self, rules: &impl ItemRules) -> u16 {
        if !self.stacks_with(other) {
            return 0;
        }
        let room = rules.max_stack(self.item).saturating_sub(self.count);
        let moved = room.min(other.count);
        self.count += moved;
        other.count -= moved;
        moved
    }

    /// Takes up to `count` items off this stack as a new one (`None` if nothing taken).
    pub fn take(&mut self, count: u16) -> Option<Self> {
        let taken = count.min(self.count);
        if taken == 0 {
            return None;
        }
        self.count -= taken;
        Some(Self {
            count: taken,
            ..self.clone()
        })
    }

    /// Takes the larger half off (a stack of one is taken whole).
    pub fn take_half(&mut self) -> Option<Self> {
        self.take(self.count.div_ceil(2))
    }

    /// Spends one use; `true` when the item broke.
    pub fn wear(&mut self, uses: u16) -> bool {
        match &mut self.durability {
            Some(left) => {
                *left = left.saturating_sub(uses);
                *left == 0
            }
            None => false,
        }
    }
}

/// Removes the stack from `slot` when it became empty.
pub fn clear_empty(slot: &mut Option<ItemStack>) {
    if slot.as_ref().is_some_and(|stack| stack.count == 0) {
        *slot = None;
    }
}
