//! The player's inventory: requests from the client, what the hand holds, what is used up.
//! Every change is sent back once per batch of messages (`take_messages`).

use super::Game;
use super::OpenWindow;
use crate::loader::ColumnLoader;
use crate::rules::BlockRules;
use strata_core::content::ItemId;
use strata_core::cooking::FurnaceSlots;
use strata_core::inventory::{
    Area, CraftGrid, HOTBAR, ItemStack, MAIN, PLAYER_SLOTS, PlayerInventory, SlotPolicy, SlotRef,
    SortOrder, Window, clear_empty, sort, wear_slot,
};
use strata_core::storage::{ItemFilter, StorageSlots};
use strata_core::upgrade::SlotOwner;
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
        if self.ecs.inventories.get(self.player).is_none() {
            return;
        }
        let mut craft = self.ecs.crafts.remove(self.player).unwrap_or_default();
        let left = self
            .with_inventory(|inventory, rules| {
                Window {
                    player: inventory,
                    open: None,
                    craft: Some(&mut craft.grid),
                    policy: None,
                }
                .close(rules)
            })
            .unwrap_or_default();
        self.throw_all(left);
        self.close_container();
        self.reset_craft();
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
            InventoryRequest::Click {
                slot:
                    SlotRef {
                        area: Area::CraftResult,
                        ..
                    },
                click,
            } => self.craft_click(loader, click),
            InventoryRequest::FillRecipe { recipe, all } => self.fill_recipe(loader, recipe, all),
            InventoryRequest::ChooseCut(choice) => self.choose_cut(loader, choice),
            InventoryRequest::ChooseUpgrade(choice) => self.choose_upgrade(choice),
            InventoryRequest::ChooseArea(index) => self.choose_area(index),
            InventoryRequest::SetFilter { slot, cell, item } => {
                self.set_filter(loader, slot, cell, item);
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
            let storage = match window.policy {
                None => window.open.as_deref().map(|open| 0..open.len()),
                Some(policy) => policy.storage(),
            };
            match (area, window.open.as_deref_mut(), storage) {
                (Area::Player, _, _) => {
                    let locked = window.player.locked.clone();
                    sort(&mut window.player.slots, MAIN, &locked, rules, key)
                }
                (Area::Open, Some(open), Some(range)) => sort(open, range, &[], rules, key),
                (Area::Open | Area::Craft | Area::CraftResult, _, _) => Vec::new(),
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

    /// The item in the selected hotbar slot; a broken one is no better than a hand.
    pub(super) fn held_item(&self) -> Option<ItemId> {
        let inventory = self.ecs.inventories.get(self.player)?;
        inventory
            .selected_stack()
            .filter(|stack| !stack.is_broken())
            .map(|stack| stack.item)
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

    /// Survival: the held item wears; at the end of its uses it breaks, or stays broken
    /// (world option).
    pub(super) fn wear_held(&mut self, uses: u16) {
        if self.is_creative() {
            return;
        }
        let mode = self.durability;
        self.with_inventory(|inventory, _| {
            wear_slot(inventory.selected_slot_mut(), uses, mode);
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
        if self.ecs.inventories.get(self.player).is_none() {
            return T::default();
        }
        let mut open = self.take_open_container(loader);
        self.check_station(loader);
        let upkeep = open
            .as_ref()
            .map(|(window, _)| self.upkeep_of(&window.halves));
        let mut craft = self.ecs.crafts.remove(self.player);
        let before = craft.as_ref().map(|window| window.grid.clone());
        let Some(player) = self.ecs.inventories.get_mut(self.player) else {
            return T::default();
        };
        let policy = open
            .as_ref()
            .and_then(|(window, _)| policy(&self.rules, window, upkeep.and_then(|u| u.filter)));
        let mut window = Window {
            player,
            open: open.as_mut().map(|(_, container)| container),
            craft: craft.as_mut().map(|window| &mut window.grid),
            policy: policy.as_ref().map(OpenPolicy::as_dyn),
        };
        let result = change(&mut window, &self.rules);
        drop(policy);
        if let Some(open) = open {
            let (halves, block) = (open.0.halves.clone(), open.0.block);
            self.put_back_open_container(open);
            self.open_storage_changed(&halves, block);
        }
        if let Some(mut craft) = craft {
            if let Some(before) = before.filter(|before| *before != craft.grid) {
                let first = |grid: &CraftGrid| grid.slots.get(0).map(|s| s.item);
                let kept = craft.bench.is_some();
                if !kept && first(&before) != first(&craft.grid) {
                    craft.cut = None;
                }
                self.craft_changed();
            }
            self.ecs.crafts.insert(self.player, craft);
        }
        self.inventory_changed();
        result
    }

    /// The screen closed: the carried stack and the crafting grid go back (or to the
    /// ground); a work station's grid gives way to the inventory's.
    pub(super) fn close_window(&mut self, loader: &ColumnLoader) {
        let left = self.with_window(loader, |window, rules| window.close(rules));
        self.throw_all(left);
        self.close_container();
        self.reset_craft();
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
        if std::mem::take(&mut self.craft_dirty)
            && let Some(view) = self.crafting_view()
        {
            self.messages.push(ServerMessage::Crafting(Box::new(view)));
        }
        std::mem::take(&mut self.messages)
    }
}

/// The rules of the open container's slots: a furnace's, or a storage's with upgrade
/// slots; none for plain storage.
enum OpenPolicy<'a> {
    Furnace(FurnaceSlots<'a>),
    Storage(StorageSlots<'a>),
}

impl OpenPolicy<'_> {
    fn as_dyn(&self) -> &dyn SlotPolicy {
        match self {
            Self::Furnace(policy) => policy,
            Self::Storage(policy) => policy,
        }
    }
}

fn policy<'a>(
    rules: &'a BlockRules,
    window: &OpenWindow,
    filter: Option<ItemFilter>,
) -> Option<OpenPolicy<'a>> {
    let layout = rules.layout(window.block)?;
    let owner = SlotOwner {
        family: layout.family.as_ref()?,
        item: rules.block_item(window.block),
    };
    if let Some(furnace) = rules.furnace(window.block) {
        return Some(OpenPolicy::Furnace(FurnaceSlots {
            cooking: &rules.cooking,
            items: rules.items(),
            upgrades: &rules.upgrades,
            kinds: &furnace.kinds,
            owner,
        }));
    }
    (layout.upgrades > 0).then(|| {
        OpenPolicy::Storage(StorageSlots {
            items: rules.items(),
            upgrades: &rules.upgrades,
            owner,
            storage: layout.storage * window.halves.len(),
            filter,
        })
    })
}
