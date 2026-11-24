//! What the upgrades of chests and barrels do: destruction of what comes in and
//! compression, sorting when the screen closes, filters on what enters. A double chest's
//! upgrades (both halves') work on its whole storage. Magnets: `magnets.rs`.

use super::{Game, OpenWindow};
use crate::loader::ColumnLoader;
use strata_core::content::ItemId;
use strata_core::coords::BlockPos;
use strata_core::inventory::{Container, ItemStack, deposit_matching};
use strata_core::storage::{
    Upkeep, after_entry, set_filter_cell, sort_storage, toggle_filter_mode,
};
use strata_core::upgrade::Effect;
use strata_protocol::ServerMessage;

impl Game {
    /// The behaviours the upgrades of these halves give their storage.
    pub(super) fn upkeep_of(&self, halves: &[BlockPos]) -> Upkeep {
        let stacks: Vec<&ItemStack> = halves
            .iter()
            .filter_map(|half| self.block_entities.get(half))
            .flat_map(|entity| entity.upgrades.slots.iter().flatten())
            .collect();
        Upkeep::new(&self.rules.upgrades, self.rules.items(), &stacks)
    }

    /// The halves of the storage at `pos` (both of a double chest), in window order.
    pub(super) fn halves_at(&self, loader: &ColumnLoader, pos: BlockPos) -> Vec<BlockPos> {
        match self.partner(loader, pos) {
            Some(other) => {
                let mut pair = vec![pos, other];
                pair.sort_by_key(|p| (p.x, p.z));
                pair
            }
            None => vec![pos],
        }
    }

    /// The storage of the halves end to end, copied.
    pub(super) fn storage_of(&self, halves: &[BlockPos]) -> Container {
        let slots = halves
            .iter()
            .filter_map(|half| self.block_entities.get(half))
            .flat_map(|entity| entity.container.slots.iter().cloned())
            .collect();
        Container { slots }
    }

    /// Runs `change` on the storage of the halves end to end, then puts it back.
    pub(super) fn with_storage<T>(
        &mut self,
        halves: &[BlockPos],
        change: impl FnOnce(&mut Container, &Self) -> T,
    ) -> T {
        let mut storage = Container::default();
        let mut sizes = Vec::new();
        for half in halves {
            let entity = self.block_entities.get_mut(half);
            let slots = entity.map(|e| std::mem::take(&mut e.container.slots));
            let slots = slots.unwrap_or_default();
            sizes.push(slots.len());
            storage.slots.extend(slots);
        }
        let result = change(&mut storage, self);
        let mut slots = storage.slots.into_iter();
        for (half, size) in halves.iter().zip(sizes) {
            if let Some(entity) = self.block_entities.get_mut(half) {
                entity.container.slots = slots.by_ref().take(size).collect();
            }
        }
        result
    }

    /// Items came into the storage of these halves since `before`: what came in and a
    /// destruction filter selects goes, compression, and sorting when `sort_now`.
    /// Returns whether it changed.
    pub(super) fn storage_entered(
        &mut self,
        halves: &[BlockPos],
        before: &Container,
        sort_now: bool,
    ) -> bool {
        let rules = self.upkeep_of(halves);
        if !rules.acts() {
            return false;
        }
        self.with_storage(halves, |storage, game| {
            let compressions = &game.rules.storage.compressions;
            after_entry(
                before,
                storage,
                &rules,
                compressions,
                game.rules.items(),
                sort_now,
            )
        })
    }

    /// The behaviours of the open window's storage (a carried bag's upgrades, or the
    /// blocks').
    pub(super) fn upkeep_of_window(&self, window: &OpenWindow) -> Upkeep {
        match window.carried {
            Some(slot) => self.carried_upkeep(slot),
            None => self.upkeep_of(&window.halves),
        }
    }

    pub(super) fn storage_of_window(&self, window: &OpenWindow) -> Container {
        match window.carried {
            Some(slot) => {
                (self.carried_contents(slot)).map_or_else(Container::default, |c| c.storage)
            }
            None => self.storage_of(&window.halves),
        }
    }

    /// `storage_entered` for the open window.
    fn window_entered(&mut self, window: &OpenWindow, before: &Container, sort_now: bool) -> bool {
        let Some(slot) = window.carried else {
            return self.storage_entered(&window.halves, before, sort_now);
        };
        let rules = self.carried_upkeep(slot);
        if !rules.acts() {
            return false;
        }
        let changed = self.with_carried(slot, |contents, game| {
            let compressions = &game.rules.storage.compressions;
            let items = game.rules.items();
            after_entry(
                before,
                &mut contents.storage,
                &rules,
                compressions,
                items,
                sort_now,
            )
        });
        changed.unwrap_or(false)
    }

    /// The open storage after a click (`before`: its storage just before): what came in
    /// is destroyed or compressed (sorting waits for the screen to close), its magnets
    /// follow its upgrade slots, and a bag's grid and tank follow its upgrades.
    pub(super) fn open_storage_changed(&mut self, window: &OpenWindow, before: Option<Container>) {
        if window.carried.is_none() && self.rules.container(window.block).is_none() {
            return;
        }
        let changed = before.is_some_and(|before| self.window_entered(window, &before, false));
        if changed {
            let contents = self.window_contents(window);
            self.messages
                .push(ServerMessage::ContainerContents(Box::new(contents)));
        }
        for &half in &window.halves {
            self.refresh_magnet(half);
        }
        if window.bag.is_some() {
            self.sync_bag_crafting(window);
            let tank = self.tank_view(window);
            self.messages.push(ServerMessage::Tank(tank));
        }
    }

    /// A quick deposit into the storage of these halves (not open): items it already
    /// holds and its filters let in; then what came in goes through its upgrades.
    pub(super) fn deposit_into(&mut self, halves: &[BlockPos]) {
        let Some(mut player) = self.ecs.inventories.remove(self.player) else {
            return;
        };
        let before = self.storage_of(halves);
        let upkeep = self.upkeep_of(halves);
        let moved = self.with_storage(halves, |storage, game| {
            let range = 0..storage.len();
            let allowed = |item: ItemId| upkeep.lets_in(item);
            deposit_matching(&mut player, storage, range, &allowed, &game.rules)
        });
        self.ecs.inventories.insert(self.player, player);
        self.inventory_changed();
        if moved > 0 {
            self.storage_entered(halves, &before, true);
        }
    }

    /// The screen of a storage closes: it sorts itself if it has a sorting upgrade.
    pub(super) fn storage_closed(&mut self, window: &OpenWindow) {
        let storage = window.carried.is_some() || self.rules.container(window.block).is_some();
        if !storage || !self.upkeep_of_window(window).sort {
            return;
        }
        match window.carried {
            Some(slot) => {
                self.with_carried(slot, |contents, game| {
                    sort_storage(&mut contents.storage, game.rules.items());
                });
            }
            None => {
                self.with_storage(&window.halves, |storage, game| {
                    sort_storage(storage, game.rules.items());
                });
            }
        }
    }

    /// Cell `cell` of the filter of the upgrade in window slot `slot` lists `item`
    /// (or nothing); with `cell` = `None`, the filter switches between allowing and
    /// refusing what it lists. Nothing already stored is touched.
    pub(super) fn set_filter(
        &mut self,
        loader: &ColumnLoader,
        slot: u16,
        cell: Option<u8>,
        item: Option<ItemId>,
    ) {
        let Some((window, mut contents)) = self.take_open_container(loader) else {
            return;
        };
        let items = self.rules.items();
        let first_upgrade = usize::from(self.window_upgrades_at(&window));
        let index = usize::from(slot);
        let filters = |stack: &ItemStack| {
            let upgrade = self.rules.upgrades.of_item(items, stack.item);
            let level = upgrade.and_then(|upgrade| upgrade.levels.first());
            level.is_some_and(|level| {
                (level.effects.iter()).any(|e| matches!(e, Effect::Filter | Effect::Void))
            })
        };
        let valid = index >= first_upgrade && contents.get(index).is_some_and(filters);
        if let Some(stack) = contents.slots.get_mut(index).and_then(Option::as_mut)
            && valid
            && item.is_none_or(|item| items.get(item).is_some())
        {
            match cell {
                Some(cell) => {
                    set_filter_cell(stack, usize::from(cell), item, items);
                }
                None => toggle_filter_mode(stack, items),
            }
        }
        let shown = window.clone();
        self.put_back_open_container((window, contents));
        self.open_storage_changed(&shown, None);
    }
}
