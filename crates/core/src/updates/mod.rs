//! Block updates: when a block changes, its six neighbours react (unsupported plants
//! drop, liquids wake up). Liquids then flow from a queue of active blocks planned for a
//! given tick: the world is never scanned, and still water stays still until disturbed.

mod fluid;
mod rules;
#[cfg(test)]
mod tests;

pub use rules::{Cell, UpdateRules};

use crate::content::{BlockId, Face};
use crate::coords::BlockPos;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub type BlockChange = (BlockPos, BlockId);

/// Ticks before a liquid waiting for an unloaded neighbour tries again.
const WAIT_TICKS: u64 = 20;

/// The blocks a simulation reads and writes (the loaded columns, on the server).
pub trait BlockAccess {
    /// `None` when the block's column is not loaded.
    fn block(&self, pos: BlockPos) -> Option<BlockId>;
    /// `false` when the block's column is not loaded.
    fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool;
    /// Inside the world (height and border).
    fn contains(&self, pos: BlockPos) -> bool {
        pos.is_in_height_range()
    }
}

/// World option: liquids may be frozen, and their cost per tick is bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FluidSettings {
    pub flow: bool,
    /// Liquid blocks updated per tick at most; the rest waits for the next tick.
    pub updates_per_tick: u32,
}

impl Default for FluidSettings {
    fn default() -> Self {
        Self {
            flow: true,
            updates_per_tick: 2048,
        }
    }
}

pub struct BlockUpdates {
    rules: Arc<UpdateRules>,
    settings: FluidSettings,
    /// Planned updates by (tick, order of planning): deterministic.
    queue: BTreeMap<(u64, u64), BlockPos>,
    planned: HashMap<BlockPos, (u64, u64)>,
    next_order: u64,
    now: u64,
}

impl BlockUpdates {
    pub fn new(rules: Arc<UpdateRules>, settings: FluidSettings) -> Self {
        Self {
            rules,
            settings,
            queue: BTreeMap::new(),
            planned: HashMap::new(),
            next_order: 0,
            now: 0,
        }
    }

    pub fn rules(&self) -> &UpdateRules {
        &self.rules
    }

    /// Turning the flow off forgets the active blocks: they stay as they are.
    pub fn set_settings(&mut self, settings: FluidSettings) {
        self.settings = settings;
        if !settings.flow {
            self.queue.clear();
            self.planned.clear();
        }
    }

    /// Blocks waiting for an update.
    pub fn active(&self) -> usize {
        self.queue.len()
    }

    /// Sets a block and lets its neighbours react; every change made is appended to
    /// `changes`. `false` (and nothing changed) when the block is not loaded.
    pub fn set_block(
        &mut self,
        world: &mut impl BlockAccess,
        pos: BlockPos,
        block: BlockId,
        changes: &mut Vec<BlockChange>,
    ) -> bool {
        if !world.set_block(pos, block) {
            return false;
        }
        let mut pending = vec![(pos, block)];
        while let Some((pos, block)) = pending.pop() {
            changes.push((pos, block));
            self.plan_fluid(pos, block);
            for face in Face::ALL {
                let neighbour = pos.neighbour(face);
                if let Some(dropped) = self.react(world, neighbour) {
                    pending.push((neighbour, dropped));
                }
            }
        }
        true
    }

    /// Advances one tick: runs the liquid updates due, within the budget.
    pub fn tick(&mut self, world: &mut impl BlockAccess, changes: &mut Vec<BlockChange>) {
        self.now += 1;
        if !self.settings.flow {
            return;
        }
        for _ in 0..self.settings.updates_per_tick {
            let Some((&(tick, order), &pos)) = self.queue.first_key_value() else {
                break;
            };
            if tick > self.now {
                break;
            }
            self.queue.remove(&(tick, order));
            self.planned.remove(&pos);
            self.update_fluid(world, pos, changes);
        }
    }

    /// A neighbour changed: liquids wake up, unsupported blocks drop (returns AIR, already
    /// written).
    fn react(&mut self, world: &mut impl BlockAccess, pos: BlockPos) -> Option<BlockId> {
        let Cell::Block(block) = cell(world, pos) else {
            return None;
        };
        self.plan_fluid(pos, block);
        if !self.rules.has_support_rule(block) {
            return None;
        }
        let below = cell(world, pos.neighbour(Face::Down));
        let above = cell(world, pos.neighbour(Face::Up));
        let supported = self.rules.is_supported(block, below, above)?;
        (!supported && world.set_block(pos, BlockId::AIR)).then_some(BlockId::AIR)
    }

    fn plan_fluid(&mut self, pos: BlockPos, block: BlockId) {
        let delay = self
            .rules
            .fluid_state(block)
            .and_then(|state| self.rules.fluid(state.source))
            .map(|fluid| u64::from(fluid.tick_rate));
        if let Some(delay) = delay {
            self.plan(pos, self.now + delay);
        }
    }

    /// Keeps the earliest of two plans for the same block.
    fn plan(&mut self, pos: BlockPos, tick: u64) {
        if !self.settings.flow {
            return;
        }
        if let Some(&old) = self.planned.get(&pos) {
            if old.0 <= tick {
                return;
            }
            self.queue.remove(&old);
        }
        let key = (tick, self.next_order);
        self.next_order += 1;
        self.queue.insert(key, pos);
        self.planned.insert(pos, key);
    }
}

/// How the simulation sees `pos` in `world`.
pub fn cell(world: &impl BlockAccess, pos: BlockPos) -> Cell {
    if !world.contains(pos) {
        return Cell::Wall;
    }
    world.block(pos).map_or(Cell::Unloaded, Cell::Block)
}
