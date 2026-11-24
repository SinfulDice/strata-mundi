//! Container blocks (chests, barrels): their contents live in block entities kept by
//! position beside the blocks; a double chest is two chests opened as one.

use super::{Game, MAX_REACH};
use crate::loader::ColumnLoader;
use glam::DVec3;
use std::collections::HashMap;
use strata_core::content::{BlockId, Face, ItemId};
use strata_core::cooking::FurnaceState;
use strata_core::coords::BlockPos;
use strata_core::inventory::{Container, ItemData, ItemStack, deposit_matching};
use strata_core::updates::BlockChange;
use strata_protocol::{OpenedContainer, ServerMessage};

/// Order in which the neighbours of a joining chest are tried.
const SIDES: [Face; 4] = [Face::West, Face::East, Face::North, Face::South];
/// Chests farther than this from the eyes are left out of a quick deposit.
const DEPOSIT_REACH: f64 = 8.0;

/// What a block with data of its own holds, apart from the block itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BlockEntity {
    /// Its own slots (a chest's grid, a furnace's input, fuel and output).
    pub container: Container,
    /// Its upgrade slots (as many as its tier gives).
    pub upgrades: Container,
    /// Room for later machines, saved with the contents.
    pub data: ItemData,
    /// Fire and cooking, for a furnace.
    pub furnace: Option<FurnaceState>,
}

/// The block entities of the dimension, by position.
pub type BlockEntities = HashMap<BlockPos, BlockEntity>;

/// The container a player has open: one block, both halves of a double chest, or a bag
/// they carry.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenWindow {
    /// For a carried bag, the block it becomes when put down (or air).
    pub block: BlockId,
    /// None for a carried bag.
    pub halves: Vec<BlockPos>,
    /// The bag opened, carried or put down.
    pub bag: Option<ItemId>,
    /// The player slot of the carried bag opened.
    pub carried: Option<usize>,
}

impl Game {
    /// Right click on a container block within reach: opens it (both halves of a
    /// double chest). Returns whether it opened.
    pub fn open_block(&mut self, loader: &ColumnLoader, pos: BlockPos) -> bool {
        let Some(block) = loader.block(pos) else {
            return false;
        };
        if let Some(station) = self.rules.station(block) {
            return self.open_station(loader, pos, block, station);
        }
        let holds = self.rules.layout(block).is_some();
        if !holds || !self.in_reach(pos) || !self.take_action() {
            return false;
        }
        let halves = match self.partner(loader, pos) {
            Some(other) => {
                let mut pair = vec![pos, other];
                pair.sort_by_key(|p| (p.x, p.z));
                pair
            }
            None => vec![pos],
        };
        self.close_container();
        let upgrades_at = self.upgrades_at(block, halves.len());
        let window = OpenWindow {
            block,
            halves,
            bag: self.rules.bags.of_block(block),
            carried: None,
        };
        let contents = self.window_contents(&window);
        let tank = self.tank_view(&window);
        self.ecs.windows.insert(self.player, window.clone());
        if self.rules.furnace(block).is_some() {
            self.wake_furnace(pos);
        }
        let furnace = self.furnace_view_at(block, pos);
        self.furnace_view = furnace;
        self.messages
            .push(ServerMessage::ContainerOpened(Box::new(OpenedContainer {
                block,
                pos,
                contents,
                upgrades_at,
                furnace,
                bag: window.bag,
                tank,
            })));
        self.sync_bag_crafting(&window);
        true
    }

    /// Index of the first upgrade slot in the window of `halves` blocks of `block`.
    pub(super) fn upgrades_at(&self, block: BlockId, halves: usize) -> u16 {
        let storage = self.rules.layout(block).map_or(0, |l| l.storage);
        u16::try_from(storage * halves).unwrap_or(u16::MAX)
    }

    /// The other half of a double chest: the first joining neighbour of the same block
    /// whose own first joining neighbour is this one (so three in a row make a pair and
    /// a single).
    pub(super) fn partner(&self, loader: &ColumnLoader, pos: BlockPos) -> Option<BlockPos> {
        self.partner_in(&|at| loader.block(at), pos)
    }

    /// `partner` in a world whose blocks `block_at` gives (what a kit would make).
    pub(super) fn partner_in(
        &self,
        block_at: &dyn Fn(BlockPos) -> Option<BlockId>,
        pos: BlockPos,
    ) -> Option<BlockPos> {
        let other = self.first_neighbour(block_at, pos)?;
        (self.first_neighbour(block_at, other) == Some(pos)).then_some(other)
    }

    fn first_neighbour(
        &self,
        block_at: &dyn Fn(BlockPos) -> Option<BlockId>,
        pos: BlockPos,
    ) -> Option<BlockPos> {
        let block = block_at(pos)?;
        if !self.rules.container(block)?.joins {
            return None;
        }
        SIDES
            .into_iter()
            .map(|side| pos.neighbour(side))
            .find(|&next| block_at(next) == Some(block))
    }

    /// Whether turning `halves` into `upgraded` would part a double chest around them
    /// (a neighbour of the new tier taking one of the halves).
    pub(super) fn would_part_a_pair(
        &self,
        loader: &ColumnLoader,
        halves: &[BlockPos],
        upgraded: BlockId,
    ) -> bool {
        let after = |at: BlockPos| {
            if halves.contains(&at) {
                Some(upgraded)
            } else {
                loader.block(at)
            }
        };
        let around = halves.iter().flat_map(|&half| {
            SIDES
                .map(|side| half.neighbour(side))
                .into_iter()
                .chain([half])
        });
        around.into_iter().any(|at| {
            let before = self.partner(loader, at);
            before.is_some() && before != self.partner_in(&after, at)
        })
    }

    /// What the window shows: the storage of each half end to end, then the upgrade
    /// slots of each half.
    pub(super) fn window_contents(&mut self, window: &OpenWindow) -> Container {
        if let Some(slot) = window.carried {
            return self
                .carried_contents(slot)
                .map(|c| c.window())
                .unwrap_or_default();
        }
        let layout = self.rules.layout(window.block).cloned();
        let (storage, upgrades) = layout.map_or((0, 0), |l| (l.storage, l.upgrades));
        let mut contents = Container::default();
        let mut slots_of_upgrades = Vec::new();
        for &half in &window.halves {
            // Made empty the first time (a chest just placed or generated); the slots
            // cut off when the pack gave it fewer fall to the ground.
            let entity = self.block_entities.entry(half).or_default();
            if self.rules.furnace(window.block).is_some() {
                entity.furnace.get_or_insert_with(Default::default);
            }
            let mut cut = entity.container.resize(storage);
            cut.extend(entity.upgrades.resize(upgrades));
            contents
                .slots
                .extend(entity.container.slots.iter().cloned());
            slots_of_upgrades.extend(entity.upgrades.slots.iter().cloned());
            for stack in cut {
                self.spawn_drop(stack, block_centre(half), DVec3::ZERO);
            }
        }
        contents.slots.extend(slots_of_upgrades);
        contents
    }

    /// Takes the open container out (both halves, end to end) for a click; `None` and
    /// the screen closes if it is gone or out of reach.
    pub(super) fn take_open_container(
        &mut self,
        loader: &ColumnLoader,
    ) -> Option<(OpenWindow, Container)> {
        let window = self.ecs.windows.get(self.player)?.clone();
        if !self.window_present(loader, &window) {
            self.close_container();
            return None;
        }
        if window.carried.is_some() {
            return self.take_carried(window);
        }
        let contents = self.window_contents(&window);
        Some((window, contents))
    }

    /// Whether what the window shows is still there: the blocks within reach, or the bag
    /// in its slot.
    pub(super) fn window_present(&self, loader: &ColumnLoader, window: &OpenWindow) -> bool {
        if let Some(slot) = window.carried {
            return self.bag_at(slot).map(|(item, _)| item) == window.bag;
        }
        window.halves.iter().all(|&half| {
            loader.block(half) == Some(window.block) && self.within(half, MAX_REACH + 1.0)
        })
    }

    /// Puts the clicked contents back into their halves and tells the client.
    pub(super) fn put_back_open_container(&mut self, (window, contents): (OpenWindow, Container)) {
        if window.carried.is_some() {
            self.put_back_carried(&window, contents);
            return;
        }
        let storage: usize = window
            .halves
            .iter()
            .filter_map(|half| self.block_entities.get(half))
            .map(|entity| entity.container.len())
            .sum();
        let (mut own, mut upgrades) = {
            let mut slots = contents.slots.clone();
            let upgrades = slots.split_off(storage.min(slots.len()));
            (slots.into_iter(), upgrades.into_iter())
        };
        for half in &window.halves {
            if let Some(entity) = self.block_entities.get_mut(half) {
                let (size, ups) = (entity.container.len(), entity.upgrades.len());
                entity.container.slots = own.by_ref().take(size).collect();
                entity.upgrades.slots = upgrades.by_ref().take(ups).collect();
            }
            if self.rules.furnace(window.block).is_some() {
                self.wake_furnace(*half);
            }
        }
        self.messages
            .push(ServerMessage::ContainerContents(Box::new(contents)));
    }

    /// Forgets the open container (the screen closed, or the chest went away).
    pub(super) fn close_container(&mut self) {
        if let Some(window) = self.ecs.windows.remove(self.player) {
            self.messages.push(ServerMessage::ContainerClosed);
            self.storage_closed(&window);
            if window.bag.is_some() {
                self.leave_bag_crafting();
            }
        }
    }

    /// Unlocked main-slot stacks go into the chests around that already hold such items
    /// and whose filter lets them in (the open one first, through the window, so its
    /// screen follows).
    pub(super) fn quick_deposit(&mut self, loader: &ColumnLoader) {
        let open: Vec<BlockPos> = self
            .ecs
            .windows
            .get(self.player)
            .map(|window| window.halves.clone())
            .unwrap_or_default();
        let mut near: Vec<(BlockPos, BlockId)> = self
            .block_entities
            .keys()
            .copied()
            .filter(|pos| !open.contains(pos) && self.within(*pos, DEPOSIT_REACH))
            .filter_map(|pos| Some((pos, loader.block(pos)?)))
            .filter(|&(_, block)| self.rules.container(block).is_some())
            .collect();
        near.sort_by_key(|&(pos, _)| (pos.x, pos.y, pos.z));
        self.with_window_entering(loader, true, |window, rules| {
            let Some(open) = window.open.as_deref_mut() else {
                return;
            };
            let (range, policy) = match window.policy {
                None => (0..open.len(), None),
                Some(policy) => match policy.storage() {
                    Some(range) => (range, Some(policy)),
                    None => return,
                },
            };
            let allowed = |item| {
                let stack = ItemStack::new(item, 1, rules);
                policy.is_none_or(|p| p.lets_in(&stack))
            };
            deposit_matching(window.player, open, range, &allowed, rules);
        });
        let mut done: Vec<BlockPos> = Vec::new();
        for (pos, _) in near {
            if done.contains(&pos) {
                continue;
            }
            let halves = self.halves_at(loader, pos);
            done.extend(&halves);
            if halves.iter().all(|half| !open.contains(half)) {
                self.deposit_into(&halves);
            }
        }
    }

    /// Restores the saved contents, made valid for the current packs (what no longer
    /// fits falls where the container is).
    pub fn set_block_entities(&mut self, entities: BlockEntities) {
        self.block_entities = entities;
        self.furnaces.clear();
        self.waiting_furnaces.clear();
        let mut thrown = Vec::new();
        for (&pos, entity) in &mut self.block_entities {
            let mut left = entity.container.normalize(&self.rules);
            left.extend(entity.upgrades.normalize(&self.rules));
            thrown.extend(left.into_iter().map(|stack| (pos, stack)));
        }
        for (pos, stack) in thrown {
            self.spawn_drop(stack, block_centre(pos), DVec3::ZERO);
        }
        let furnaces: Vec<BlockPos> = self
            .block_entities
            .iter()
            .filter(|(_, entity)| entity.furnace.is_some())
            .map(|(&pos, _)| pos)
            .collect();
        for pos in furnaces {
            self.wake_furnace(pos);
        }
        self.find_magnets();
    }

    /// A container block or a furnace that went away (broken, burnt, or replaced by a
    /// block holding another kind of entity) throws its contents where it was.
    pub(super) fn settle_block_entities(&mut self, loader: &ColumnLoader, changes: &[BlockChange]) {
        for &(pos, block) in changes {
            let kept = self.block_entities.get(&pos).is_some_and(|entity| {
                self.rules.layout(block).is_some()
                    && entity.furnace.is_some() == self.rules.furnace(block).is_some()
            });
            if !kept && let Some(mut entity) = self.block_entities.remove(&pos) {
                self.magnets.at.remove(&pos);
                let centre = block_centre(pos);
                if let Some(bag) = self.bag_taken_up(&mut entity) {
                    self.spawn_drop(bag, centre, DVec3::ZERO);
                    continue;
                }
                let mut stacks = entity.container.drain();
                stacks.extend(entity.upgrades.drain());
                for stack in stacks {
                    self.spawn_drop(stack, centre, DVec3::ZERO);
                }
            }
        }
        let open = self.ecs.windows.get(self.player).cloned();
        let broken = open.is_some_and(|window| {
            window
                .halves
                .iter()
                .any(|&half| loader.block(half) != Some(window.block))
        });
        if broken {
            self.close_container();
        }
        self.check_station(loader);
    }

    pub(super) fn within(&self, pos: BlockPos, distance: f64) -> bool {
        self.player_body()
            .is_some_and(|body| body.eyes().distance(block_centre(pos)) <= distance)
    }
}

pub(super) fn block_centre(pos: BlockPos) -> DVec3 {
    DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)) + 0.5
}
