//! What a click in an inventory screen does. Pure functions over the player's inventory
//! and the open container, so the server applies them and tests check them.

use super::container::{ARMOR, Container, HOTBAR, MAIN, OFFHAND, PLAYER_SLOTS, PlayerInventory};
use super::stack::{ItemRules, ItemStack, clear_empty};
use serde::{Deserialize, Serialize};

/// Which part of the screen a slot belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Area {
    Player,
    /// The container the player opened (a chest, both halves of a double chest).
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SlotRef {
    pub area: Area,
    pub index: u16,
}

impl SlotRef {
    pub const fn player(index: usize) -> Self {
        Self {
            area: Area::Player,
            index: index as u16,
        }
    }

    pub const fn open(index: usize) -> Self {
        Self {
            area: Area::Open,
            index: index as u16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Click {
    /// Left click (A): take the stack, put the carried one, or exchange them.
    Primary,
    /// Right click (X): take half, or put one.
    Secondary,
    /// Shift click (Y): send the stack to the other side.
    Transfer,
    /// Double click: gather the same items onto the carried stack.
    Gather,
    /// Number key over a slot: exchange with that hotbar slot.
    HotbarSwap(u8),
    /// Drop key over a slot: one item, or the whole stack.
    Drop { whole: bool },
    /// Click outside the window: drop the carried stack, or one of it.
    DropCursor { whole: bool },
}

/// The player's inventory and the container they opened, if any.
pub struct Window<'a> {
    pub player: &'a mut PlayerInventory,
    pub open: Option<&'a mut Container>,
}

impl Window<'_> {
    /// Applies a click; returns the stacks thrown into the world.
    pub fn click(&mut self, slot: SlotRef, click: Click, rules: &impl ItemRules) -> Vec<ItemStack> {
        if !self.exists(slot) {
            return Vec::new();
        }
        match click {
            Click::Primary => self.primary(slot, rules),
            Click::Secondary => self.secondary(slot, rules),
            Click::Transfer => self.transfer(slot, rules),
            Click::Gather => self.gather(rules),
            Click::HotbarSwap(hotbar) => self.hotbar_swap(slot, usize::from(hotbar), rules),
            Click::Drop { whole } => return self.drop_from(slot, whole).into_iter().collect(),
            Click::DropCursor { whole } => return self.drop_cursor(whole).into_iter().collect(),
        }
        Vec::new()
    }

    fn exists(&self, slot: SlotRef) -> bool {
        let index = usize::from(slot.index);
        match slot.area {
            Area::Player => index < PLAYER_SLOTS,
            Area::Open => self.open.as_ref().is_some_and(|open| index < open.len()),
        }
    }

    fn slot_mut(&mut self, slot: SlotRef) -> Option<&mut Option<ItemStack>> {
        let index = usize::from(slot.index);
        match slot.area {
            Area::Player => self.player.slots.slots.get_mut(index),
            Area::Open => self.open.as_mut()?.slots.get_mut(index),
        }
    }

    fn accepts(&self, slot: SlotRef, stack: &ItemStack, rules: &impl ItemRules) -> bool {
        match slot.area {
            Area::Player => self.player.accepts(usize::from(slot.index), stack, rules),
            Area::Open => true,
        }
    }

    fn primary(&mut self, slot: SlotRef, rules: &impl ItemRules) {
        let carried = self.player.cursor.take();
        let fits = carried
            .as_ref()
            .is_none_or(|c| self.accepts(slot, c, rules));
        let Some(target) = self.slot_mut(slot) else {
            return;
        };
        self.player.cursor = match (carried, target.take()) {
            (None, held) => held,
            (Some(carried), held) if !fits => {
                *target = held;
                Some(carried)
            }
            (Some(carried), None) => {
                *target = Some(carried);
                None
            }
            (Some(mut carried), Some(mut held)) if held.stacks_with(&carried) => {
                held.absorb(&mut carried, rules);
                *target = Some(held);
                (carried.count > 0).then_some(carried)
            }
            (Some(carried), Some(held)) => {
                *target = Some(carried);
                Some(held)
            }
        };
    }

    fn secondary(&mut self, slot: SlotRef, rules: &impl ItemRules) {
        let Some(mut carried) = self.player.cursor.take() else {
            let target = self.slot_mut(slot);
            let taken = target.and_then(|target| {
                let half = target.as_mut()?.take_half();
                clear_empty(target);
                half
            });
            self.player.cursor = taken;
            return;
        };
        let fits = self.accepts(slot, &carried, rules);
        let Some(target) = self.slot_mut(slot) else {
            self.player.cursor = Some(carried);
            return;
        };
        match target {
            _ if !fits => {}
            None => *target = carried.take(1),
            Some(held) if held.stacks_with(&carried) => {
                if let Some(mut one) = carried.take(1) {
                    held.absorb(&mut one, rules);
                    carried.count += one.count;
                }
            }
            Some(held) => std::mem::swap(held, &mut carried),
        }
        self.player.cursor = (carried.count > 0).then_some(carried);
    }

    /// Shift click: into the open container, or out of it; without one, between the
    /// hotbar and the main slots (armour goes to its slot first).
    fn transfer(&mut self, slot: SlotRef, rules: &impl ItemRules) {
        let Some(stack) = self.slot_mut(slot).and_then(Option::take) else {
            return;
        };
        let index = usize::from(slot.index);
        let left = match (slot.area, &mut self.open) {
            (Area::Open, _) => self.player.give(stack, rules),
            (Area::Player, Some(open)) => open.insert_anywhere(stack, rules),
            (Area::Player, None) => self.move_within_player(index, stack, rules),
        };
        if let Some(target) = self.slot_mut(slot) {
            *target = left;
        }
    }

    fn move_within_player(
        &mut self,
        from: usize,
        stack: ItemStack,
        rules: &impl ItemRules,
    ) -> Option<ItemStack> {
        let player = &mut *self.player;
        let armour = ARMOR.clone().find(|&index| {
            !ARMOR.contains(&from)
                && player.accepts(index, &stack, rules)
                && player.slots.get(index).is_none()
        });
        if let Some(index) = armour {
            player.slots.slots[index] = Some(stack);
            return None;
        }
        let slots = &mut player.slots;
        if HOTBAR.contains(&from) {
            slots.insert(stack, MAIN, rules)
        } else if MAIN.contains(&from) {
            slots.insert(stack, HOTBAR, rules)
        } else {
            slots.insert(stack, MAIN.chain(HOTBAR), rules)
        }
    }

    /// Fills the carried stack from the unlocked slots, the partial stacks first.
    fn gather(&mut self, rules: &impl ItemRules) {
        let Some(mut carried) = self.player.cursor.take() else {
            return;
        };
        let max = rules.max_stack(carried.item);
        for full in [false, true] {
            for slot in self.gather_order() {
                if carried.count >= max {
                    break;
                }
                let Some(target) = self.slot_mut(slot) else {
                    continue;
                };
                if let Some(held) = target
                    .as_mut()
                    .filter(|held| held.stacks_with(&carried) && (held.count >= max) == full)
                {
                    carried.absorb(held, rules);
                    clear_empty(target);
                }
            }
        }
        self.player.cursor = Some(carried);
    }

    fn gather_order(&self) -> Vec<SlotRef> {
        let open = self.open.as_ref().map_or(0, |open| open.len());
        let player = MAIN.chain(HOTBAR).filter(|&i| !self.player.is_locked(i));
        (0..open)
            .map(SlotRef::open)
            .chain(player.map(SlotRef::player))
            .collect()
    }

    fn hotbar_swap(&mut self, slot: SlotRef, hotbar: usize, rules: &impl ItemRules) {
        let quick = SlotRef::player(hotbar);
        if !HOTBAR.contains(&hotbar) || slot == quick {
            return;
        }
        let here = self.slot_mut(slot).and_then(Option::take);
        let there = self.player.slots.slots[hotbar].take();
        let allowed = there.as_ref().is_none_or(|s| self.accepts(slot, s, rules));
        let (here, there) = if allowed {
            (there, here)
        } else {
            (here, there)
        };
        self.player.slots.slots[hotbar] = there;
        if let Some(target) = self.slot_mut(slot) {
            *target = here;
        }
    }

    fn drop_from(&mut self, slot: SlotRef, whole: bool) -> Option<ItemStack> {
        let target = self.slot_mut(slot)?;
        let stack = target.as_mut()?;
        let dropped = if whole {
            stack.take(stack.count)
        } else {
            stack.take(1)
        };
        clear_empty(target);
        dropped
    }

    fn drop_cursor(&mut self, whole: bool) -> Option<ItemStack> {
        let carried = self.player.cursor.as_mut()?;
        let dropped = if whole {
            carried.take(carried.count)
        } else {
            carried.take(1)
        };
        clear_empty(&mut self.player.cursor);
        dropped
    }

    /// Closing the screen: the carried stack goes back into the inventory; what does not
    /// fit is returned, to be dropped.
    pub fn close(&mut self, rules: &impl ItemRules) -> Option<ItemStack> {
        let carried = self.player.cursor.take()?;
        let player = &mut *self.player;
        player
            .slots
            .insert(carried, HOTBAR.chain(MAIN).chain([OFFHAND]), rules)
    }
}
