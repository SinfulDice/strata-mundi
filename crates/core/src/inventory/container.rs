use super::stack::{ItemRules, ItemStack, clear_empty};
use crate::content::{EquipSlot, ItemId};
use serde::{Deserialize, Serialize};
use std::ops::Range;

pub const HOTBAR: Range<usize> = 0..9;
pub const MAIN: Range<usize> = 9..36;
/// Head, chest, legs, feet (`EquipSlot::ARMOR` order).
pub const ARMOR: Range<usize> = 36..40;
pub const OFFHAND: usize = 40;
/// Two rings, an amulet, a belt, a cloak (`EquipSlot::ACCESSORIES` order); saves from
/// before them have 41 slots and get these empty.
pub const ACCESSORIES: Range<usize> = 41..46;
/// The backpack worn on the back; saves from before it have 46 slots and get it empty.
pub const BACK: usize = 46;
pub const PLAYER_SLOTS: usize = 47;

/// The slots whose stack is worn: armour, then accessories.
pub fn worn_slots() -> impl Iterator<Item = usize> + Clone {
    ARMOR.chain(ACCESSORIES)
}

/// The kind of piece slot `index` takes, for armour and accessory slots.
pub fn slot_equip(index: usize) -> Option<EquipSlot> {
    if ARMOR.contains(&index) {
        return EquipSlot::ARMOR.get(index - ARMOR.start).copied();
    }
    if ACCESSORIES.contains(&index) {
        return EquipSlot::ACCESSORIES
            .get(index - ACCESSORIES.start)
            .copied();
    }
    (index == BACK).then_some(EquipSlot::Back)
}

/// A fixed number of slots (a chest, a barrel, the player's inventory).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub slots: Vec<Option<ItemStack>>,
}

impl Container {
    pub fn new(size: usize) -> Self {
        Self {
            slots: vec![None; size],
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(Option::is_none)
    }

    pub fn get(&self, index: usize) -> Option<&ItemStack> {
        self.slots.get(index)?.as_ref()
    }

    pub fn contains(&self, item: ItemId) -> bool {
        self.slots.iter().flatten().any(|stack| stack.item == item)
    }

    pub fn count(&self, item: ItemId) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|stack| stack.item == item)
            .map(|stack| u32::from(stack.count))
            .sum()
    }

    /// Puts `stack` into the slots of `order`: joins the matching stacks first, then
    /// fills the first empty slot. Returns what did not fit.
    pub fn insert(
        &mut self,
        mut stack: ItemStack,
        order: impl Iterator<Item = usize> + Clone,
        rules: &impl ItemRules,
    ) -> Option<ItemStack> {
        for index in order.clone() {
            if let Some(Some(existing)) = self.slots.get_mut(index) {
                existing.absorb(&mut stack, rules);
                if stack.count == 0 {
                    return None;
                }
            }
        }
        for index in order {
            if let Some(slot @ None) = self.slots.get_mut(index) {
                let max = rules.max_stack(stack.item);
                let Some(placed) = stack.take(max) else {
                    return Some(stack);
                };
                *slot = Some(placed);
                if stack.count == 0 {
                    return None;
                }
            }
        }
        Some(stack)
    }

    /// Everything goes in, in slot order.
    pub fn insert_anywhere(
        &mut self,
        stack: ItemStack,
        rules: &impl ItemRules,
    ) -> Option<ItemStack> {
        self.insert(stack, 0..self.len(), rules)
    }

    /// Makes every slot hold a valid stack (a save from other packs may not): unknown or
    /// empty stacks go, over-full ones are split into the free slots. Returns what does
    /// not fit, to be thrown.
    pub fn normalize(&mut self, rules: &impl ItemRules) -> Vec<ItemStack> {
        let mut extra = Vec::new();
        for slot in &mut self.slots {
            let Some(stack) = slot else {
                continue;
            };
            let max = rules.max_stack(stack.item);
            if max == 0 || stack.count == 0 {
                *slot = None;
            } else if stack.count > max {
                extra.extend(stack.take(stack.count - max));
            }
        }
        extra
            .into_iter()
            .filter_map(|stack| self.insert_anywhere(stack, rules))
            .collect()
    }

    /// Keeps the first `size` slots; the stacks of the others are returned.
    pub fn resize(&mut self, size: usize) -> Vec<ItemStack> {
        let cut = if self.slots.len() > size {
            self.slots.split_off(size).into_iter().flatten().collect()
        } else {
            Vec::new()
        };
        self.slots.resize(size, None);
        cut
    }

    /// Takes all the stacks out (breaking the block that held them).
    pub fn drain(&mut self) -> Vec<ItemStack> {
        self.slots.iter_mut().filter_map(Option::take).collect()
    }

    /// Removes `count` items of `item`, from the last slots first; returns how many.
    pub fn remove(&mut self, item: ItemId, count: u16) -> u16 {
        let mut left = count;
        for slot in self.slots.iter_mut().rev() {
            if let Some(stack) = slot.as_mut().filter(|stack| stack.item == item) {
                left -= stack.take(left).map_or(0, |taken| taken.count);
                clear_empty(slot);
            }
            if left == 0 {
                break;
            }
        }
        count - left
    }
}

/// The player's 46 slots (hotbar, main, armour, off hand, accessories), the selected hotbar slot,
/// the locked slots and the stack carried under the pointer while a screen is open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInventory {
    pub slots: Container,
    /// Indexed like the slots; locked ones keep their place when sorting and are left
    /// out of quick deposits and transfers of whole rows.
    pub locked: Vec<bool>,
    pub selected: u8,
    pub cursor: Option<ItemStack>,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            slots: Container::new(PLAYER_SLOTS),
            locked: vec![false; PLAYER_SLOTS],
            selected: 0,
            cursor: None,
        }
    }
}

impl PlayerInventory {
    pub fn selected_index(&self) -> usize {
        usize::from(self.selected).min(HOTBAR.end - 1)
    }

    pub fn selected_stack(&self) -> Option<&ItemStack> {
        self.slots.get(self.selected_index())
    }

    pub fn selected_slot_mut(&mut self) -> &mut Option<ItemStack> {
        let index = self.selected_index();
        &mut self.slots.slots[index]
    }

    pub fn is_locked(&self, index: usize) -> bool {
        self.locked.get(index).copied().unwrap_or(false)
    }

    /// Locks or unlocks a hotbar or main slot; others cannot be locked.
    pub fn toggle_lock(&mut self, index: usize) -> bool {
        let lockable = HOTBAR.contains(&index) || MAIN.contains(&index);
        if let (true, Some(lock)) = (lockable, self.locked.get_mut(index)) {
            *lock = !*lock;
        }
        lockable
    }

    /// Whether `stack` may lie in slot `index` (armour and accessory slots take only
    /// their piece).
    pub fn accepts(index: usize, stack: &ItemStack, rules: &impl ItemRules) -> bool {
        match slot_equip(index) {
            Some(equip) => rules.equip(stack.item) == Some(equip),
            None => index < PLAYER_SLOTS,
        }
    }

    /// Picks something up: the hotbar first, then the main slots. Returns what did not fit.
    pub fn give(&mut self, stack: ItemStack, rules: &impl ItemRules) -> Option<ItemStack> {
        self.slots.insert(stack, HOTBAR.chain(MAIN), rules)
    }

    /// Makes a restored inventory valid: current number of slots, stacks within their
    /// limit, armour and accessories only in their slot, the carried stack put back. What was moved out
    /// goes into the hotbar and main slots; what does not fit is returned, to be thrown.
    pub fn sanitize(&mut self, rules: &impl ItemRules) -> Vec<ItemStack> {
        self.normalize();
        let mut moved: Vec<ItemStack> = self.cursor.take().into_iter().collect();
        for (index, slot) in self.slots.slots.iter_mut().enumerate() {
            let Some(stack) = slot else {
                continue;
            };
            let max = rules.max_stack(stack.item);
            if max == 0 || stack.count == 0 {
                *slot = None;
            } else if !Self::accepts(index, stack, rules) {
                moved.extend(slot.take());
            } else if stack.count > max {
                moved.extend(stack.take(stack.count - max));
            }
        }
        moved
            .into_iter()
            .filter_map(|stack| self.give(stack, rules))
            .collect()
    }

    /// Brings a newer or older save to the current number of slots.
    pub fn normalize(&mut self) {
        self.slots.slots.resize(PLAYER_SLOTS, None);
        self.locked.resize(PLAYER_SLOTS, false);
        self.selected = self.selected.min(HOTBAR.end as u8 - 1);
    }
}
