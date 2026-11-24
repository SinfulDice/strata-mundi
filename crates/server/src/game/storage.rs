//! What the upgrades of chests and barrels do: destruction of what comes in and
//! compression, sorting when the screen closes, filters on what enters. A double chest's
//! upgrades (both halves') work on its whole storage. Magnets: `magnets.rs`.

use super::Game;
use crate::loader::ColumnLoader;
use strata_core::content::{BlockId, ItemId};
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

    /// The open storage after a click (`before`: its storage just before): what came in
    /// is destroyed or compressed (sorting waits for the screen to close), and its magnets
    /// follow its upgrade slots.
    pub(super) fn open_storage_changed(
        &mut self,
        halves: &[BlockPos],
        block: BlockId,
        before: Option<Container>,
    ) {
        if self.rules.container(block).is_none() {
            return;
        }
        let changed = before.is_some_and(|before| self.storage_entered(halves, &before, false));
        if changed && let Some(window) = self.ecs.windows.get(self.player).cloned() {
            let contents = self.window_contents(&window);
            self.messages
                .push(ServerMessage::ContainerContents(Box::new(contents)));
        }
        for &half in halves {
            self.refresh_magnet(half);
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
    pub(super) fn storage_closed(&mut self, halves: &[BlockPos], block: BlockId) {
        if self.rules.container(block).is_some() && self.upkeep_of(halves).sort {
            self.with_storage(halves, |storage, game| {
                sort_storage(storage, game.rules.items())
            });
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
        let storage = self.rules.layout(window.block).map_or(0, |l| l.storage);
        let first_upgrade = storage * window.halves.len();
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
        let halves = window.halves.clone();
        let block = window.block;
        self.put_back_open_container((window, contents));
        self.open_storage_changed(&halves, block, None);
    }
}
