//! The player's inventory: requests from the client, what the hand holds, what is used up.
//! Every change is sent back once per batch of messages (`take_messages`).

use super::Game;
use crate::loader::ColumnLoader;
use strata_core::content::ItemId;
use strata_core::inventory::{
    Area, HOTBAR, ItemStack, MAIN, PLAYER_SLOTS, PlayerInventory, SortOrder, Window, clear_empty,
    sort,
};
use strata_protocol::InventoryRequest;

impl Game {
    pub fn inventory(&self) -> PlayerInventory {
        self.ecs
            .inventories
            .get(self.player)
            .cloned()
            .unwrap_or_default()
    }

    /// Restores a saved inventory: unknown or impossible stacks are left out.
    pub fn set_inventory(&mut self, mut inventory: PlayerInventory) {
        inventory.normalize();
        for (index, slot) in inventory.slots.slots.iter_mut().enumerate() {
            let fits = slot.as_ref().is_some_and(|stack| {
                stack.is_valid(&self.rules) && PlayerInventory::accepts(index, stack, &self.rules)
            });
            if !fits {
                *slot = None;
            }
        }
        inventory.cursor = None;
        self.ecs.inventories.insert(self.player, inventory);
        self.inventory_changed();
    }

    /// Applies a request; `names` gives an item's name in the request's language.
    pub fn inventory_request(
        &mut self,
        loader: &ColumnLoader,
        request: InventoryRequest,
        names: &dyn Fn(ItemId, &str) -> String,
    ) {
        match request {
            InventoryRequest::Select(slot) => {
                if usize::from(slot) < HOTBAR.end {
                    self.with_inventory(|inventory| inventory.selected = slot);
                }
            }
            InventoryRequest::Click { slot, click } => {
                let rules = self.rules.clone();
                let thrown = self.with_window(loader, |window| window.click(slot, click, &rules));
                self.throw_all(thrown);
            }
            InventoryRequest::Close => self.close_window(loader),
            InventoryRequest::Sort {
                area,
                order,
                language,
            } => self.sort(loader, area, order, &|item| names(item, &language)),
            InventoryRequest::ToggleLock(index) => {
                self.with_inventory(|inventory| inventory.toggle_lock(usize::from(index)));
            }
            InventoryRequest::QuickDeposit => self.quick_deposit(loader),
            InventoryRequest::CreativeSet { index, stack } => self.creative_set(index, stack),
            InventoryRequest::DropSelected { whole } => {
                let thrown = self.with_inventory(|inventory| {
                    let slot = inventory.selected_slot_mut();
                    let stack = slot.as_mut()?;
                    let taken = stack.take(if whole { stack.count } else { 1 });
                    clear_empty(slot);
                    taken
                });
                self.throw_all(thrown.flatten());
            }
        }
    }

    fn creative_set(&mut self, index: u16, stack: Option<ItemStack>) {
        let index = usize::from(index);
        let valid = index < PLAYER_SLOTS
            && self.is_creative()
            && stack.as_ref().is_none_or(|stack| {
                stack.is_valid(&self.rules) && PlayerInventory::accepts(index, stack, &self.rules)
            });
        if valid {
            self.with_inventory(|inventory| inventory.slots.slots[index] = stack);
        }
    }

    fn sort(
        &mut self,
        loader: &ColumnLoader,
        area: Area,
        order: SortOrder,
        name: &dyn Fn(ItemId) -> String,
    ) {
        let rules = self.rules.clone();
        let key = |item: ItemId| match order {
            SortOrder::Type => (
                rules
                    .item_category(item)
                    .map_or_else(String::new, ToString::to_string),
                item.0,
            ),
            SortOrder::Name => (name(item).to_lowercase(), item.0),
        };
        self.with_window(loader, |window| match (area, window.open.as_deref_mut()) {
            (Area::Player, _) => {
                let locked = window.player.locked.clone();
                sort(&mut window.player.slots, MAIN, &locked, &rules, key);
            }
            (Area::Open, Some(open)) => {
                let range = 0..open.len();
                sort(open, range, &[], &rules, key);
            }
            (Area::Open, None) => {}
        });
    }

    /// Picks up `stack`; returns what did not fit.
    pub fn give(&mut self, stack: ItemStack) -> Option<ItemStack> {
        let rules = self.rules.clone();
        let Some(inventory) = self.ecs.inventories.get_mut(self.player) else {
            return Some(stack);
        };
        let left = inventory.give(stack, &rules);
        self.inventory_changed();
        left
    }

    /// The item in the selected hotbar slot.
    pub(super) fn held_item(&self) -> Option<ItemId> {
        let inventory = self.ecs.inventories.get(self.player)?;
        inventory.selected_stack().map(|stack| stack.item)
    }

    /// Survival: one held item is used up (a block placed, a boat put down).
    pub(super) fn consume_held(&mut self) {
        if self.is_creative() {
            return;
        }
        self.with_inventory(|inventory| {
            let slot = inventory.selected_slot_mut();
            if let Some(stack) = slot.as_mut() {
                stack.count -= 1;
            }
            clear_empty(slot);
        });
    }

    /// Survival: the held item wears; it breaks at the end of its uses.
    pub(super) fn wear_held(&mut self, uses: u16) {
        if self.is_creative() {
            return;
        }
        self.with_inventory(|inventory| {
            let slot = inventory.selected_slot_mut();
            if slot.as_mut().is_some_and(|stack| stack.wear(uses)) {
                *slot = None;
            }
        });
    }

    /// Survival: one held item becomes `item` (a bucket filled or emptied); the new one
    /// takes the slot if it was the last, or goes in the inventory (or to the ground).
    pub(super) fn exchange_held(&mut self, item: ItemId) {
        if self.is_creative() {
            return;
        }
        let new = ItemStack::new(item, 1, &self.rules);
        let rules = self.rules.clone();
        let left = self.with_inventory(|inventory| {
            let slot = inventory.selected_slot_mut();
            match slot.as_mut() {
                Some(stack) if stack.count > 1 => {
                    stack.count -= 1;
                    inventory.give(new, &rules)
                }
                _ => {
                    *slot = Some(new);
                    None
                }
            }
        });
        self.throw_all(left.flatten());
    }

    /// Runs `change` on the player's inventory and sends it afterwards.
    pub(super) fn with_inventory<T>(
        &mut self,
        change: impl FnOnce(&mut PlayerInventory) -> T,
    ) -> Option<T> {
        let inventory = self.ecs.inventories.get_mut(self.player)?;
        let result = change(inventory);
        self.inventory_changed();
        Some(result)
    }

    /// Runs `change` on the player's inventory and the open container (if any), then
    /// sends both.
    pub(super) fn with_window<T: Default>(
        &mut self,
        loader: &ColumnLoader,
        change: impl FnOnce(&mut Window<'_>) -> T,
    ) -> T {
        let mut open = self.take_open_container(loader);
        let Some(player) = self.ecs.inventories.get_mut(self.player) else {
            return T::default();
        };
        let mut window = Window {
            player,
            open: open.as_mut().map(|(_, container)| container),
        };
        let result = change(&mut window);
        if let Some(open) = open {
            self.put_back_open_container(open);
        }
        self.inventory_changed();
        result
    }

    /// The screen closed: the carried stack goes back (or to the ground).
    fn close_window(&mut self, loader: &ColumnLoader) {
        let rules = self.rules.clone();
        let left = self.with_window(loader, |window| window.close(&rules));
        self.throw_all(left);
        self.close_container();
    }

    pub(super) fn inventory_changed(&mut self) {
        self.inventory_dirty = true;
    }
}
