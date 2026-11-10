//! Block updates: when a block changes, its six neighbours react (unsupported plants
//! drop, liquids wake up). Liquids then flow from a queue of active blocks planned for a
//! given tick: the world is never scanned, and still water stays still until disturbed.
//! Fires share the queue and its budget (one fire step weighs three liquid steps): they
//! age, spread and burn out.

mod fire;
#[cfg(test)]
mod fire_tests;
mod fluid;
mod rules;
#[cfg(test)]
mod tests;

pub use fire::FireSettings;
pub use rules::{Cell, UpdateRules};

use crate::content::{BlockId, Face};
use crate::coords::BlockPos;
use crate::dimension::DimensionId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

pub type BlockChange = (BlockPos, BlockId);

/// Ticks before a liquid waiting for an unloaded neighbour tries again.
const WAIT_TICKS: u64 = 20;
/// A fire step reads its whole box of air: measured at about three liquid steps
/// (`fire/prairie_fire_40_ticks` against `fluids/large_flow_settles`), so it counts as
/// three in the budget.
const FIRE_UPDATE_COST: u32 = 3;

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
    /// Rain falls on this block and puts out fire there. No weather before v0.5.
    fn rains_on(&self, _pos: BlockPos) -> bool {
        false
    }
}

/// World option: liquids may be frozen, and their cost per tick is bounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FluidSettings {
    pub flow: bool,
    /// Liquid steps per tick at most (a fire step counts as `FIRE_UPDATE_COST`); the rest
    /// waits for the next tick.
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
    fire: FireSettings,
    /// World seed mixed with the dimension: each world and dimension burns its own way.
    seed: u64,
    /// Age of each burning fire (0 to 15). When its column unloads, the fire's planned
    /// step is dropped and its age forgotten; `wake_fires` starts it again on reload.
    fire_ages: HashMap<BlockPos, u8>,
    /// Planned updates by (tick, order of planning): deterministic.
    queue: BTreeMap<(u64, u64), BlockPos>,
    planned: HashMap<BlockPos, (u64, u64)>,
    next_order: u64,
    now: u64,
    /// Changes still to propagate, kept to reuse its memory.
    pending: Vec<BlockChange>,
}

impl BlockUpdates {
    pub fn new(rules: Arc<UpdateRules>, settings: FluidSettings) -> Self {
        Self {
            rules,
            settings,
            fire: FireSettings::default(),
            seed: 0,
            fire_ages: HashMap::new(),
            queue: BTreeMap::new(),
            planned: HashMap::new(),
            next_order: 0,
            now: 0,
            pending: Vec::new(),
        }
    }

    pub fn rules(&self) -> &UpdateRules {
        &self.rules
    }

    /// Turning the flow off forgets the active liquids: they stay as they are.
    pub fn set_settings(&mut self, settings: FluidSettings) {
        self.settings = settings;
    }

    pub fn set_fire_settings(&mut self, settings: FireSettings) {
        self.fire = settings;
    }

    pub fn set_seed(&mut self, world_seed: u64, dimension: &DimensionId) {
        self.seed = world_seed ^ stable_hash(&dimension.to_string());
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
        let mut pending = std::mem::take(&mut self.pending);
        pending.push((pos, block));
        while let Some((pos, block)) = pending.pop() {
            changes.push((pos, block));
            self.fire_ages.remove(&pos);
            self.plan_block(pos, block);
            for face in Face::ALL {
                let neighbour = pos.neighbour(face);
                if let Some(dropped) = self.react(world, neighbour) {
                    pending.push((neighbour, dropped));
                }
            }
        }
        self.pending = pending;
        true
    }

    /// Advances one tick: runs the liquid and fire updates due, within the budget.
    pub fn tick(&mut self, world: &mut impl BlockAccess, changes: &mut Vec<BlockChange>) {
        self.now += 1;
        let mut spent = 0;
        while spent < self.settings.updates_per_tick {
            spent += 1;
            let Some((&(tick, order), &pos)) = self.queue.first_key_value() else {
                break;
            };
            if tick > self.now {
                break;
            }
            self.queue.remove(&(tick, order));
            self.planned.remove(&pos);
            match cell(world, pos) {
                Cell::Block(block) if self.rules.fire(block).is_some() => {
                    spent += FIRE_UPDATE_COST - 1;
                    self.update_fire(world, pos, changes);
                }
                Cell::Block(_) if self.settings.flow => self.update_fluid(world, pos, changes),
                _ => {
                    self.fire_ages.remove(&pos);
                }
            }
        }
    }

    /// A neighbour changed: liquids and fires wake up, unsupported blocks drop and fires
    /// without fuel or next to water go out (returns AIR, already written).
    fn react(&mut self, world: &mut impl BlockAccess, pos: BlockPos) -> Option<BlockId> {
        let Cell::Block(block) = cell(world, pos) else {
            return None;
        };
        self.plan_block(pos, block);
        if self.rules.fire(block).is_some() {
            let out = self.fire_can_stay(world, pos) == Some(false);
            return (out && world.set_block(pos, BlockId::AIR)).then_some(BlockId::AIR);
        }
        if !self.rules.has_support_rule(block) {
            return None;
        }
        let below = cell(world, pos.neighbour(Face::Down));
        let above = cell(world, pos.neighbour(Face::Up));
        let supported = self.rules.is_supported(block, below, above)?;
        (!supported && world.set_block(pos, BlockId::AIR)).then_some(BlockId::AIR)
    }

    fn plan_block(&mut self, pos: BlockPos, block: BlockId) {
        if let Some(fire) = self.rules.fire(block) {
            self.plan_fire(pos, fire);
        } else if self.settings.flow {
            self.plan_fluid(pos, block);
        }
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

/// FNV-1a: the same on every machine and Rust version, unlike `DefaultHasher`.
fn stable_hash(text: &str) -> u64 {
    text.bytes().fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
    })
}

/// How the simulation sees `pos` in `world`.
pub fn cell(world: &impl BlockAccess, pos: BlockPos) -> Cell {
    if !world.contains(pos) {
        return Cell::Wall;
    }
    world.block(pos).map_or(Cell::Unloaded, Cell::Block)
}
