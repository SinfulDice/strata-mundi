//! Bags opened from the inventory: their contents live in the item's data, read and
//! written back around every click; the bag's slot is pinned while its screen is open, so
//! the item never moves away from its contents. A crafting upgrade gives the screen a 3×3
//! grid, a tank upgrade a tank filled and emptied with buckets.

use super::crafting::CraftWindow;
use super::{Game, OpenWindow};
use crate::loader::ColumnLoader;
use strata_core::bag::{BagContents, BagRules, TankUse, bag_slots, use_tank};
use strata_core::content::{BlockId, ItemId};
use strata_core::coords::BlockPos;
use strata_core::inventory::{Area, Click, Container, CraftGrid, ItemStack, SlotRef};
use strata_core::storage::Upkeep;
use strata_protocol::{OpenedContainer, ServerMessage, TankView};

impl Game {
    /// The bag in player slot `slot`, and its rules.
    pub(super) fn bag_at(&self, slot: usize) -> Option<(ItemId, BagRules)> {
        let stack = self.ecs.inventories.get(self.player)?.slots.get(slot)?;
        Some((stack.item, self.rules.bags.get(stack.item)?.clone()))
    }

    /// What the bag in `slot` holds (what a pack no longer gives a slot for left out).
    pub(super) fn carried_contents(&self, slot: usize) -> Option<BagContents> {
        let (_, rules) = self.bag_at(slot)?;
        let stack = self.ecs.inventories.get(self.player)?.slots.get(slot)?;
        Some(BagContents::read(&stack.data, &rules.layout).0)
    }

    pub(super) fn carried_upkeep(&self, slot: usize) -> Upkeep {
        let contents = self.carried_contents(slot).unwrap_or_default();
        let upgrades: Vec<&ItemStack> = contents.upgrades.slots.iter().flatten().collect();
        Upkeep::new(&self.rules.upgrades, self.rules.items(), &upgrades)
    }

    /// Runs `change` on what the bag in `slot` holds, then writes it back into the item.
    pub(super) fn with_carried<T>(
        &mut self,
        slot: usize,
        change: impl FnOnce(&mut BagContents, &Self) -> T,
    ) -> Option<T> {
        let (_, rules) = self.bag_at(slot)?;
        let stack = self.ecs.inventories.get(self.player)?.slots.get(slot)?;
        let (mut contents, cut) = BagContents::read(&stack.data, &rules.layout);
        let result = change(&mut contents, self);
        let inventory = self.ecs.inventories.get_mut(self.player)?;
        let stack = inventory.slots.slots.get_mut(slot)?.as_mut()?;
        contents.write(&mut stack.data);
        self.inventory_changed();
        self.throw_all(cut);
        Some(result)
    }

    /// The carried bag's slots, taken out of the item for a click (`put_back_carried`
    /// puts them back): the item never holds a copy of what is being moved.
    pub(super) fn take_carried(&mut self, window: OpenWindow) -> Option<(OpenWindow, Container)> {
        let slot = window.carried?;
        let taken = self.with_carried(slot, |contents, _| {
            let shown = contents.window();
            contents.storage = Container::new(contents.storage.len());
            contents.upgrades = Container::new(contents.upgrades.len());
            shown
        })?;
        Some((window, taken))
    }

    pub(super) fn put_back_carried(&mut self, window: &OpenWindow, shown: Container) {
        let slot = window.carried.unwrap_or(usize::MAX);
        let mut left = Some(shown.clone());
        if self.window_present_carried(window) {
            self.with_carried(slot, |contents, _| {
                contents.set_window(left.take().unwrap_or_default())
            });
        }
        // Only if the bag went away during the click: what it held falls, never lost.
        let lost: Vec<ItemStack> = left.into_iter().flat_map(|c| c.slots).flatten().collect();
        self.throw_all(lost);
        self.messages
            .push(ServerMessage::ContainerContents(Box::new(shown)));
    }

    fn window_present_carried(&self, window: &OpenWindow) -> bool {
        let slot = window.carried.unwrap_or(usize::MAX);
        self.bag_at(slot).map(|(item, _)| item) == window.bag
    }

    /// Index of the first upgrade slot in the window.
    pub(super) fn window_upgrades_at(&self, window: &OpenWindow) -> u16 {
        match window.bag.and_then(|bag| self.rules.bags.get(bag)) {
            Some(rules) if window.carried.is_some() => {
                u16::try_from(rules.layout.storage).unwrap_or(u16::MAX)
            }
            _ => self.upgrades_at(window.block, window.halves.len()),
        }
    }

    /// Opens the bag in player slot `slot` (its key, or a right click with it in hand).
    pub(super) fn open_bag(&mut self, loader: &ColumnLoader, slot: u16) {
        let slot = usize::from(slot);
        let Some((item, rules)) = bag_slots()
            .any(|s| s == slot)
            .then(|| self.bag_at(slot))
            .flatten()
        else {
            return;
        };
        self.close_window(loader);
        let window = OpenWindow {
            block: rules.placed.unwrap_or(BlockId::AIR),
            halves: Vec::new(),
            bag: Some(item),
            carried: Some(slot),
        };
        let contents = self.window_contents(&window);
        let tank = self.tank_view(&window);
        self.ecs.windows.insert(self.player, window.clone());
        self.furnace_view = None;
        let opened = OpenedContainer {
            block: window.block,
            pos: BlockPos::new(0, 0, 0),
            contents,
            upgrades_at: self.window_upgrades_at(&window),
            furnace: None,
            bag: Some(item),
            tank,
        };
        self.messages
            .push(ServerMessage::ContainerOpened(Box::new(opened)));
        self.sync_bag_crafting(&window);
    }

    /// A bag with a crafting upgrade lends its 3×3 grid to the screen; without one, the
    /// grid goes (what it held back to the inventory).
    pub(super) fn sync_bag_crafting(&mut self, window: &OpenWindow) {
        let wants = window.bag.is_some() && self.upkeep_of_window(window).crafting;
        let has = self.has_bag_grid();
        if wants && !has {
            self.reset_craft();
            let grid = CraftWindow {
                grid: CraftGrid::new(3),
                ..CraftWindow::default()
            };
            self.ecs.crafts.insert(self.player, grid);
            self.craft_changed();
        } else if !wants && has {
            self.reset_craft();
        }
    }

    /// The bag's screen closed: its grid gives way to the inventory's.
    pub(super) fn leave_bag_crafting(&mut self) {
        if self.has_bag_grid() {
            self.reset_craft();
        }
    }

    fn has_bag_grid(&self) -> bool {
        self.ecs.crafts.get(self.player).is_some_and(|craft| {
            craft.station.is_none() && craft.grid.side == 3 && craft.bench.is_none()
        })
    }

    /// The open bag's tank, if it has a tank upgrade.
    pub(super) fn tank_view(&self, window: &OpenWindow) -> Option<TankView> {
        window.bag?;
        let capacity = self.upkeep_of_window(window).tank?;
        let tank = match window.carried {
            Some(slot) => self.carried_contents(slot).and_then(|c| c.tank),
            None => (window.halves.first())
                .and_then(|half| self.block_entities.get(half))
                .and_then(|entity| BagContents::read_tank(&entity.data)),
        };
        Some(TankView {
            bucket: tank
                .as_ref()
                .and_then(|t| self.rules.items().key(&t.bucket)),
            buckets: tank.map_or(0, |t| t.buckets),
            capacity,
        })
    }

    /// The carried bucket and the open bag's tank: a full bucket pours in, an empty one
    /// fills (one at a time).
    pub(super) fn tank_click(&mut self, loader: &ColumnLoader) {
        let Some(window) = self.ecs.windows.get(self.player).cloned() else {
            return;
        };
        let capacity = self.upkeep_of_window(&window).tank;
        let (Some(capacity), true) = (capacity, self.window_present(loader, &window)) else {
            return;
        };
        let mut contents = BagContents {
            tank: self.tank_of(&window),
            ..BagContents::default()
        };
        let Some(inventory) = self.ecs.inventories.get_mut(self.player) else {
            return;
        };
        let mut cursor = inventory.cursor.take();
        let used = use_tank(&mut contents, capacity, &mut cursor, self.rules.items());
        inventory.cursor = cursor;
        self.inventory_changed();
        if used == TankUse::Nothing {
            return;
        }
        self.set_tank_of(&window, contents.tank.as_ref());
        if let TankUse::Filled(Some(full)) = used {
            let left = self.give(full);
            self.throw_all(left);
        }
        let view = self.tank_view(&window);
        self.messages.push(ServerMessage::Tank(view));
    }

    fn tank_of(&self, window: &OpenWindow) -> Option<strata_core::bag::Tank> {
        match window.carried {
            Some(slot) => self.carried_contents(slot)?.tank,
            None => BagContents::read_tank(&self.block_entities.get(window.halves.first()?)?.data),
        }
    }

    fn set_tank_of(&mut self, window: &OpenWindow, tank: Option<&strata_core::bag::Tank>) {
        match window.carried {
            Some(slot) => {
                let tank = tank.cloned();
                self.with_carried(slot, |contents, _| contents.tank = tank);
            }
            None => {
                let entity = window
                    .halves
                    .first()
                    .and_then(|h| self.block_entities.get_mut(h));
                if let Some(entity) = entity {
                    BagContents::write_tank(&mut entity.data, tank);
                }
            }
        }
    }

    /// The player slot of the carried bag whose screen is open.
    pub(super) fn pinned(&self) -> Option<usize> {
        self.ecs.windows.get(self.player)?.carried
    }

    pub(super) fn selected(&self) -> usize {
        self.ecs
            .inventories
            .get(self.player)
            .map_or(0, |inventory| inventory.selected_index())
    }

    /// Whether a click would move the open bag (taking it, swapping it with a number key).
    pub(super) fn touches_pinned(&self, slot: SlotRef, click: Click) -> bool {
        let Some(pinned) = self.pinned() else {
            return false;
        };
        let on_it = slot.area == Area::Player && usize::from(slot.index) == pinned;
        on_it || click == Click::HotbarSwap(u8::try_from(pinned).unwrap_or(u8::MAX))
    }

    /// Runs `change` with the open bag's slot locked (sorting and quick deposits leave
    /// locked slots alone).
    pub(super) fn with_bag_pinned(&mut self, change: impl FnOnce(&mut Self)) {
        let pinned = self.pinned();
        let was = pinned.and_then(|slot| self.lock_slot(slot, true));
        change(self);
        if let (Some(slot), Some(was)) = (pinned, was) {
            self.lock_slot(slot, was);
        }
    }

    /// Sets the lock of `slot`; returns what it was.
    fn lock_slot(&mut self, slot: usize, locked: bool) -> Option<bool> {
        let lock = self
            .ecs
            .inventories
            .get_mut(self.player)?
            .locked
            .get_mut(slot)?;
        Some(std::mem::replace(lock, locked))
    }

    /// Whether `stack` holds no bag (no bag in a bag, whatever a client sends).
    pub(super) fn holds_no_bag(&self, stack: &ItemStack) -> bool {
        let Some(rules) = self.rules.bags.get(stack.item) else {
            return true;
        };
        let (contents, _) = BagContents::read(&stack.data, &rules.layout);
        contents
            .stacks()
            .all(|inner| !self.rules.bags.is_bag(inner.item))
    }
}
