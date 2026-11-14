//! Container blocks (chests, barrels): their contents live in block entities kept by
//! position beside the blocks; a double chest is two chests opened as one.

use super::{Game, MAX_REACH};
use crate::loader::ColumnLoader;
use glam::DVec3;
use std::collections::HashMap;
use strata_core::content::{BlockId, Face};
use strata_core::coords::BlockPos;
use strata_core::inventory::{Container, ItemData, deposit_matching};
use strata_core::updates::BlockChange;
use strata_protocol::{OpenedContainer, ServerMessage};

/// Order in which the neighbours of a joining chest are tried.
const SIDES: [Face; 4] = [Face::West, Face::East, Face::North, Face::South];
/// Chests farther than this from the eyes are left out of a quick deposit.
const DEPOSIT_REACH: f64 = 8.0;

/// What a block with data of its own holds, apart from the block itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BlockEntity {
    pub container: Container,
    /// Room for later machines (furnace progress…), saved with the contents.
    pub data: ItemData,
}

/// The block entities of the dimension, by position.
pub type BlockEntities = HashMap<BlockPos, BlockEntity>;

/// The container a player has open: one block, or both halves of a double chest.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenWindow {
    pub block: BlockId,
    pub halves: Vec<BlockPos>,
}

impl Game {
    /// Right click on a container block within reach: opens it (both halves of a
    /// double chest). Returns whether it opened.
    pub fn open_block(&mut self, loader: &ColumnLoader, pos: BlockPos) -> bool {
        let Some(block) = loader.block(pos) else {
            return false;
        };
        if self.rules.container(block).is_none() || !self.in_reach(pos) || !self.take_action() {
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
        let window = OpenWindow { block, halves };
        let contents = self.contents(&window);
        self.ecs.windows.insert(self.player, window);
        self.messages
            .push(ServerMessage::ContainerOpened(Box::new(OpenedContainer {
                block,
                pos,
                contents,
            })));
        true
    }

    /// The other half of a double chest: the first joining neighbour of the same block
    /// whose own first joining neighbour is this one (so three in a row make a pair and
    /// a single).
    pub(super) fn partner(&self, loader: &ColumnLoader, pos: BlockPos) -> Option<BlockPos> {
        let other = self.first_neighbour(loader, pos)?;
        (self.first_neighbour(loader, other) == Some(pos)).then_some(other)
    }

    fn first_neighbour(&self, loader: &ColumnLoader, pos: BlockPos) -> Option<BlockPos> {
        let block = loader.block(pos)?;
        if !self.rules.container(block)?.joins {
            return None;
        }
        SIDES
            .into_iter()
            .map(|side| pos.neighbour(side))
            .find(|&next| loader.block(next) == Some(block))
    }

    fn contents(&mut self, window: &OpenWindow) -> Container {
        let mut contents = Container::default();
        for &half in &window.halves {
            let size = self
                .rules
                .container(window.block)
                .map_or(0, |def| def.slots());
            let entity = self.block_entity(half, size);
            contents
                .slots
                .extend(entity.container.slots.iter().cloned());
        }
        contents
    }

    /// The block entity at `pos`, made empty the first time (a chest just placed or
    /// generated), resized if the pack changed its rows.
    fn block_entity(&mut self, pos: BlockPos, size: usize) -> &mut BlockEntity {
        let entity = self.block_entities.entry(pos).or_default();
        entity.container.slots.resize(size, None);
        entity
    }

    /// Takes the open container out (both halves, end to end) for a click; `None` and
    /// the screen closes if it is gone or out of reach.
    pub(super) fn take_open_container(
        &mut self,
        loader: &ColumnLoader,
    ) -> Option<(OpenWindow, Container)> {
        let window = self.ecs.windows.get(self.player)?.clone();
        let present = window.halves.iter().all(|&half| {
            loader.block(half) == Some(window.block) && self.within(half, MAX_REACH + 1.0)
        });
        if !present {
            self.close_container();
            return None;
        }
        let contents = self.contents(&window);
        Some((window, contents))
    }

    /// Puts the clicked contents back into their halves and tells the client.
    pub(super) fn put_back_open_container(&mut self, (window, contents): (OpenWindow, Container)) {
        let mut slots = contents.slots.clone().into_iter();
        for half in &window.halves {
            if let Some(entity) = self.block_entities.get_mut(half) {
                let size = entity.container.len();
                entity.container.slots = slots.by_ref().take(size).collect();
            }
        }
        self.messages
            .push(ServerMessage::ContainerContents(Box::new(contents)));
    }

    /// Forgets the open container (the screen closed, or the chest went away).
    pub(super) fn close_container(&mut self) {
        if self.ecs.windows.remove(self.player).is_some() {
            self.messages.push(ServerMessage::ContainerClosed);
        }
    }

    /// Unlocked main-slot stacks go into the chests around that already hold such items.
    pub(super) fn quick_deposit(&mut self, loader: &ColumnLoader) {
        let rules = self.rules.clone();
        let mut near: Vec<BlockPos> = self
            .block_entities
            .keys()
            .copied()
            .filter(|&pos| self.within(pos, DEPOSIT_REACH))
            .filter(|&pos| {
                loader
                    .block(pos)
                    .is_some_and(|b| rules.container(b).is_some())
            })
            .collect();
        near.sort_by_key(|&pos| (pos.x, pos.y, pos.z));
        let Some(player) = self.ecs.inventories.get_mut(self.player) else {
            return;
        };
        let mut moved = 0;
        for pos in near {
            if let Some(entity) = self.block_entities.get_mut(&pos) {
                moved += deposit_matching(player, &mut entity.container, &rules);
            }
        }
        if moved > 0 {
            self.inventory_changed();
            if let Some((window, _)) = self.take_open_container(loader) {
                let contents = self.contents(&window);
                self.put_back_open_container((window, contents));
            }
        }
    }

    /// A container block that went away (broken, burnt) throws its contents where it was.
    pub(super) fn settle_block_entities(&mut self, loader: &ColumnLoader, changes: &[BlockChange]) {
        for &(pos, block) in changes {
            let gone = self.rules.container(block).is_none();
            if let (true, Some(mut entity)) = (gone, self.block_entities.remove(&pos)) {
                let centre = block_centre(pos);
                for stack in entity.container.drain() {
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
    }

    fn within(&self, pos: BlockPos, distance: f64) -> bool {
        self.player_body()
            .is_some_and(|body| body.eyes().distance(block_centre(pos)) <= distance)
    }
}

pub(super) fn block_centre(pos: BlockPos) -> DVec3 {
    DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)) + 0.5
}
