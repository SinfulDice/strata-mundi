//! The player's inventory: requests from the client, what the hand holds, what is used up.
//! Every change is sent back once per batch of messages (`take_messages`).

use super::Game;
use crate::loader::ColumnLoader;
use crate::rules::BlockRules;
use strata_core::content::ItemId;
use strata_core::inventory::{
    Area, HOTBAR, ItemStack, MAIN, PLAYER_SLOTS, PlayerInventory, SortOrder, Window, clear_empty,
    sort,
};
use strata_protocol::{InventoryRequest, ServerMessage};

impl Game {
    pub fn inventory(&self) -> PlayerInventory {
        self.ecs
            .inventories
            .get(self.player)
            .cloned()
            .unwrap_or_default()
    }

    /// Restores a saved inventory, made valid for the current packs; what has no place
    /// any more (an over-full stack, a misplaced piece of armour) is thrown.
    pub fn set_inventory(&mut self, mut inventory: PlayerInventory) {
        let thrown = inventory.sanitize(&self.rules);
        self.ecs.inventories.insert(self.player, inventory);
        self.inventory_changed();
        self.throw_all(thrown);
    }

    /// The player leaves: the carried stack goes back into the inventory (or to the
    /// ground) and the open container closes.
    pub fn release_cursor(&mut self) {
        let left = self
            .with_inventory(|inventory, rules| {
                Window {
                    player: inventory,
                    open: None,
                }
                .close(rules)
            })
            .flatten();
        self.throw_all(left);
        self.close_container();
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
                    self.with_inventory(|inventory, _| inventory.selected = slot);
                }
            }
            InventoryRequest::Click { slot, click } => {
                let thrown =
                    self.with_window(loader, |window, rules| window.click(slot, click, rules));
                self.throw_all(thrown);
            }
            InventoryRequest::Close => self.close_window(loader),
            InventoryRequest::Sort {
                area,
                order,
                language,
            } => self.sort(loader, area, order, &|item| names(item, &language)),
            InventoryRequest::ToggleLock(index) => {
                self.with_inventory(|inventory, _| inventory.toggle_lock(usize::from(index)));
            }
            InventoryRequest::QuickDeposit => self.quick_deposit(loader),
            InventoryRequest::CreativeSet { index, stack } => self.creative_set(index, stack),
            InventoryRequest::DropSelected { whole } => {
                let thrown = self.with_inventory(|inventory, _| {
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
            self.with_inventory(|inventory, _| inventory.slots.slots[index] = stack);
        }
    }

    fn sort(
        &mut self,
        loader: &ColumnLoader,
        area: Area,
        order: SortOrder,
        name: &dyn Fn(ItemId) -> String,
    ) {
        let left = self.with_window(loader, |window, rules| {
            let key = |item: ItemId| match order {
                SortOrder::Type => (
                    rules
                        .item_category(item)
                        .map_or_else(String::new, ToString::to_string),
                    item.0,
                ),
                SortOrder::Name => (name(item).to_lowercase(), item.0),
            };
            match (area, window.open.as_deref_mut()) {
                (Area::Player, _) => {
                    let locked = window.player.locked.clone();
                    sort(&mut window.player.slots, MAIN, &locked, rules, key)
                }
                (Area::Open, Some(open)) => {
                    let range = 0..open.len();
                    sort(open, range, &[], rules, key)
                }
                (Area::Open, None) => Vec::new(),
            }
        });
        self.throw_all(left);
    }

    /// Picks up `stack`; returns what did not fit. The inventory is sent only if
    /// something went in.
    pub fn give(&mut self, stack: ItemStack) -> Option<ItemStack> {
        let Some(inventory) = self.ecs.inventories.get_mut(self.player) else {
            return Some(stack);
        };
        let count = stack.count;
        let left = inventory.give(stack, &self.rules);
        if left.as_ref().is_none_or(|left| left.count < count) {
            self.inventory_changed();
        }
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
        self.with_inventory(|inventory, _| {
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
        self.with_inventory(|inventory, _| {
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
        let left = self.with_inventory(|inventory, rules| {
            let slot = inventory.selected_slot_mut();
            match slot.as_mut() {
                Some(stack) if stack.count == 1 => {
                    *slot = Some(new);
                    None
                }
                Some(stack) => {
                    stack.count -= 1;
                    inventory.give(new, rules)
                }
                None => None,
            }
        });
        self.throw_all(left.flatten());
    }

    /// Runs `change` on the player's inventory and sends it afterwards.
    pub(super) fn with_inventory<T>(
        &mut self,
        change: impl FnOnce(&mut PlayerInventory, &BlockRules) -> T,
    ) -> Option<T> {
        let inventory = self.ecs.inventories.get_mut(self.player)?;
        let result = change(inventory, &self.rules);
        self.inventory_changed();
        Some(result)
    }

    /// Runs `change` on the player's inventory and the open container (if any), then
    /// sends both.
    pub(super) fn with_window<T: Default>(
        &mut self,
        loader: &ColumnLoader,
        change: impl FnOnce(&mut Window<'_>, &BlockRules) -> T,
    ) -> T {
        let mut open = self.take_open_container(loader);
        let Some(player) = self.ecs.inventories.get_mut(self.player) else {
            return T::default();
        };
        let mut window = Window {
            player,
            open: open.as_mut().map(|(_, container)| container),
        };
        let result = change(&mut window, &self.rules);
        if let Some(open) = open {
            self.put_back_open_container(open);
        }
        self.inventory_changed();
        result
    }

    /// The screen closed: the carried stack goes back (or to the ground).
    fn close_window(&mut self, loader: &ColumnLoader) {
        let left = self.with_window(loader, |window, rules| window.close(rules));
        self.throw_all(left);
        self.close_container();
    }

    pub(super) fn inventory_changed(&mut self) {
        self.inventory_dirty = true;
    }

    /// Entity, container and inventory news since the last call.
    pub fn take_messages(&mut self) -> Vec<ServerMessage> {
        if std::mem::take(&mut self.inventory_dirty) {
            let inventory = Box::new(self.inventory());
            self.messages.push(ServerMessage::Inventory(inventory));
        }
        std::mem::take(&mut self.messages)
    }
}
