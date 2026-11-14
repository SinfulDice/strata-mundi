//! Comfort: sorting a range of slots and quick deposit into chests.

use super::container::{Container, MAIN, PlayerInventory};
use super::stack::{ItemRules, ItemStack};
use crate::content::ItemId;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortOrder {
    /// By category, then in pack order.
    #[default]
    Type,
    /// By translated name.
    Name,
}

/// Joins the equal stacks of `range` and lays them out in order, skipping the locked
/// slots (which keep what they hold). `key` ranks an item.
pub fn sort<K: Ord>(
    slots: &mut Container,
    range: Range<usize>,
    locked: &[bool],
    rules: &impl ItemRules,
    key: impl Fn(ItemId) -> K,
) {
    let free: Vec<usize> = range
        .filter(|&i| i < slots.len() && !locked.get(i).copied().unwrap_or(false))
        .collect();
    let mut stacks: Vec<ItemStack> = free.iter().filter_map(|&i| slots.slots[i].take()).collect();
    stacks.sort_by_key(|stack| std::cmp::Reverse(stack.count));
    stacks.sort_by_cached_key(|stack| key(stack.item));
    let mut packed = Container::new(free.len());
    for stack in stacks {
        let left = packed.insert_anywhere(stack, rules);
        debug_assert!(left.is_none(), "packing never needs more slots");
    }
    for (slot, stack) in free.into_iter().zip(packed.slots) {
        slots.slots[slot] = stack;
    }
}

/// Moves the unlocked main-slot stacks of items that `chest` already holds into it.
/// Returns how many items moved.
pub fn deposit_matching(
    player: &mut PlayerInventory,
    chest: &mut Container,
    rules: &impl ItemRules,
) -> u32 {
    let mut moved = 0;
    for index in MAIN {
        if player.is_locked(index) {
            continue;
        }
        let slot = &mut player.slots.slots[index];
        let Some(stack) = slot.take_if(|stack| chest.contains(stack.item)) else {
            continue;
        };
        let count = stack.count;
        *slot = chest.insert_anywhere(stack, rules);
        moved += u32::from(count - slot.as_ref().map_or(0, |left| left.count));
    }
    moved
}
