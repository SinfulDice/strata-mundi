//! What the upgrades of chests and barrels do: destruction and compression when items
//! come in, sorting when the screen closes, filters on what enters, magnets drawing the
//! items lying around. A double chest's upgrades (both halves') work on its whole storage.

use super::Game;
use super::containers::block_centre;
use super::drops::DROP_SIZE;
use crate::ecs::Entity;
use crate::loader::ColumnLoader;
use std::collections::BTreeMap;
use strata_core::content::BlockId;
use strata_core::coords::BlockPos;
use strata_core::inventory::{Container, ItemStack, deposit_matching};
use strata_core::storage::{Upkeep, set_filter_cell, take_in, toggle_filter_mode, upkeep};
use strata_core::upgrade::Effect;
use strata_protocol::ServerMessage;

/// Chests and barrels with a magnet, and whether magnets work at all (world option).
#[derive(Debug, Clone, Default)]
pub struct Magnets {
    /// Radius and period of each storage with a magnet upgrade.
    pub at: BTreeMap<BlockPos, (f32, u16)>,
    pub disabled: bool,
}

impl Game {
    /// World option: storage magnets draw items lying around.
    pub fn set_storage_magnets(&mut self, enabled: bool) {
        self.magnets.disabled = !enabled;
    }

    /// The behaviours the upgrades of these halves give their storage.
    pub(super) fn upkeep_of(&self, halves: &[BlockPos]) -> Upkeep {
        let stacks: Vec<&ItemStack> = halves
            .iter()
            .filter_map(|half| self.block_entities.get(half))
            .flat_map(|entity| entity.upgrades.slots.iter().flatten())
            .collect();
        Upkeep::new(&self.rules.upgrades, self.rules.items(), &stacks)
    }

    /// Runs `change` on the storage of the halves end to end, then puts it back.
    fn with_storage<T>(
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

    /// Items came into the storage of these halves: destruction and compression, and
    /// sorting when `closed`. Returns whether it changed.
    pub(super) fn storage_upkeep(&mut self, halves: &[BlockPos], closed: bool) -> bool {
        let rules = self.upkeep_of(halves);
        if !rules.acts() {
            return false;
        }
        self.with_storage(halves, |storage, game| {
            let compressions = &game.rules.storage.compressions;
            upkeep(storage, &rules, compressions, game.rules.items(), closed)
        })
    }

    /// The open storage after a click: what came in is destroyed or compressed (sorting
    /// waits for the screen to close).
    pub(super) fn open_storage_changed(&mut self, halves: &[BlockPos], block: BlockId) {
        if self.rules.container(block).is_none() {
            return;
        }
        if self.storage_upkeep(halves, false)
            && let Some(window) = self.ecs.windows.get(self.player).cloned()
        {
            let contents = self.window_contents(&window);
            self.messages
                .push(ServerMessage::ContainerContents(Box::new(contents)));
        }
        for &half in halves {
            self.refresh_magnet(half);
        }
    }

    /// A quick deposit into the storage at `pos` (not open): items it already holds and
    /// its filter lets in.
    pub(super) fn deposit_into(&mut self, pos: BlockPos, block: BlockId) {
        let Some(player) = self.ecs.inventories.get_mut(self.player) else {
            return;
        };
        let rules = &self.rules;
        let Some(entity) = self.block_entities.get_mut(&pos) else {
            return;
        };
        let stacks: Vec<&ItemStack> = entity.upgrades.slots.iter().flatten().collect();
        let upkeep = Upkeep::new(&rules.upgrades, rules.items(), &stacks);
        let range = 0..entity.container.len();
        let allowed = |item| upkeep.lets_in(item);
        let moved = deposit_matching(player, &mut entity.container, range, &allowed, rules);
        if moved > 0 && rules.container(block).is_some() {
            self.storage_upkeep(&[pos], true);
        }
    }

    /// The upgrade slots of the storage at `pos` may have changed: its magnet with them.
    pub(super) fn refresh_magnet(&mut self, pos: BlockPos) {
        match self.upkeep_of(&[pos]).magnet {
            Some(magnet) => self.magnets.at.insert(pos, magnet),
            None => self.magnets.at.remove(&pos),
        };
    }

    #[cfg(test)]
    pub(crate) fn has_magnet(&self, pos: BlockPos) -> bool {
        self.magnets.at.contains_key(&pos)
    }

    /// After loading: the storages with a magnet.
    pub(super) fn find_magnets(&mut self) {
        self.magnets.at.clear();
        let positions: Vec<BlockPos> = self.block_entities.keys().copied().collect();
        for pos in positions {
            self.refresh_magnet(pos);
        }
    }

    /// Each magnet, at its period, draws the items lying within its radius (those the
    /// player could pick up) into its storage, if the filter lets them in.
    pub(super) fn magnets_tick(&mut self, loader: &ColumnLoader) {
        if self.magnets.disabled || self.magnets.at.is_empty() || self.ecs.drops.len() == 0 {
            return;
        }
        let due: Vec<(BlockPos, f32)> = (self.magnets.at.iter())
            .filter(|(_, (_, period))| self.tick.is_multiple_of(u64::from(*period)))
            .map(|(&pos, &(radius, _))| (pos, radius))
            .collect();
        for (pos, radius) in due {
            let present = loader
                .block(pos)
                .is_some_and(|block| self.rules.container(block).is_some());
            if !present {
                continue;
            }
            let halves = match self.partner(loader, pos) {
                Some(other) => vec![pos, other],
                None => vec![pos],
            };
            if self.draw_items(&halves, pos, f64::from(radius)) {
                self.storage_changed_on_its_own(&halves);
            }
        }
    }

    fn draw_items(&mut self, halves: &[BlockPos], pos: BlockPos, radius: f64) -> bool {
        let centre = block_centre(pos);
        let near: Vec<Entity> = (self.ecs.drops.iter())
            .filter(|(_, drop)| drop.pickup_delay == 0)
            .filter(|(_, drop)| {
                let middle = drop.body.position + glam::DVec3::Y * (DROP_SIZE / 2.0);
                middle.distance(centre) <= radius
            })
            .map(|(entity, _)| entity)
            .collect();
        let upkeep = self.upkeep_of(halves);
        let mut drew = false;
        for entity in near {
            let Some(stack) = self.ecs.drops.get(entity).map(|d| d.stack.clone()) else {
                continue;
            };
            let left = self.with_storage(halves, |storage, game| {
                take_in(storage, stack.clone(), &upkeep, game.rules.items())
            });
            match left {
                None => self.remove_drop(entity),
                Some(left) if left != stack => {
                    if let Some(drop) = self.ecs.drops.get_mut(entity) {
                        drop.stack = left;
                    }
                }
                Some(_) => continue,
            }
            drew = true;
        }
        drew
    }

    /// The storage of these halves took items in on its own (magnet): upkeep, sorting
    /// unless its screen is open, and the screen follows.
    fn storage_changed_on_its_own(&mut self, halves: &[BlockPos]) {
        let window = self.ecs.windows.get(self.player).cloned();
        let open = window
            .as_ref()
            .is_some_and(|w| w.halves.iter().any(|h| halves.contains(h)));
        self.storage_upkeep(halves, !open);
        if let Some(window) = window.filter(|_| open) {
            let contents = self.window_contents(&window);
            self.messages
                .push(ServerMessage::ContainerContents(Box::new(contents)));
        }
    }

    /// The screen of a storage closes: it sorts itself if it has a sorting upgrade.
    pub(super) fn storage_closed(&mut self, halves: &[BlockPos], block: BlockId) {
        if self.rules.container(block).is_some() {
            self.storage_upkeep(halves, true);
        }
    }

    /// Cell `cell` of the filter of the upgrade in window slot `slot` lists `item`
    /// (or nothing); with `cell` = `None`, the filter switches between allowing and
    /// refusing what it lists.
    pub(super) fn set_filter(
        &mut self,
        loader: &ColumnLoader,
        slot: u16,
        cell: Option<u8>,
        item: Option<strata_core::content::ItemId>,
    ) {
        let Some((window, mut contents)) = self.take_open_container(loader) else {
            return;
        };
        let items = self.rules.items();
        let storage = self.rules.layout(window.block).map_or(0, |l| l.storage);
        let first_upgrade = storage * window.halves.len();
        let index = usize::from(slot);
        let filters = |stack: &ItemStack| {
            let bricks = self.rules.upgrades.of_item(items, stack.item);
            let level = bricks.and_then(|upgrade| upgrade.levels.first());
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
        self.open_storage_changed(&halves, block);
    }
}
