//! The slots of a furnace: what each one takes, and where a shift click sends a stack.

use super::Cooking;
use crate::content::{Item, ItemId, Registry};
use crate::id::Identifier;
use crate::inventory::{Container, ItemStack, SlotPolicy};
use crate::upgrade::{SlotOwner, Upgrades};

pub const FURNACE_INPUT: usize = 0;
pub const FURNACE_FUEL: usize = 1;
pub const FURNACE_OUTPUT: usize = 2;
/// The upgrade slots follow, as many as the tier has.
pub const FURNACE_MODULES: usize = 3;

pub struct FurnaceSlots<'a> {
    pub cooking: &'a Cooking,
    pub items: &'a Registry<ItemId, Item>,
    pub upgrades: &'a Upgrades,
    pub kinds: &'a [Identifier],
    /// The family of its upgrade slots and the furnace's item.
    pub owner: SlotOwner<'a>,
}

impl FurnaceSlots<'_> {
    fn is_module(&self, item: ItemId) -> bool {
        self.items.get(item).is_some_and(|i| i.upgrade.is_some())
    }

    /// An upgrade slot takes an upgrade that suits the furnace and is not already in
    /// another slot.
    fn module_fits(&self, slots: &Container, index: usize, stack: &ItemStack) -> bool {
        let others = (FURNACE_MODULES..slots.len())
            .filter(|&other| other != index)
            .filter_map(|other| slots.get(other));
        (self.upgrades).fits_slot(self.items, self.owner, others, stack)
    }
}

impl SlotPolicy for FurnaceSlots<'_> {
    fn accepts(&self, slots: &Container, index: usize, stack: &ItemStack) -> bool {
        match index {
            FURNACE_INPUT => true,
            FURNACE_FUEL => self.cooking.fuel(stack.item).is_some(),
            FURNACE_OUTPUT => false,
            _ => self.module_fits(slots, index, stack),
        }
    }

    /// Something it cooks goes to the input, a fuel to the fuel slot, a module to a free
    /// upgrade slot; anything else stays where it is.
    fn route(&self, slots: &Container, stack: &ItemStack) -> Vec<usize> {
        if self.is_module(stack.item) {
            return (FURNACE_MODULES..slots.len())
                .filter(|&i| slots.get(i).is_none() && self.module_fits(slots, i, stack))
                .take(1)
                .collect();
        }
        let cooks = self.cooking.recipe_for(stack.item, self.kinds).is_some();
        let burns = self.cooking.fuel(stack.item).is_some();
        let input = slots
            .get(FURNACE_INPUT)
            .is_none_or(|held| held.stacks_with(stack));
        match (cooks, burns) {
            (true, true) if !input => vec![FURNACE_FUEL],
            (true, _) => vec![FURNACE_INPUT],
            (false, true) => vec![FURNACE_FUEL],
            (false, false) => Vec::new(),
        }
    }
}
