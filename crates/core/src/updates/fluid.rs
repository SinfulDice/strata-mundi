//! One step of a liquid block, close to Minecraft's: fill level from the neighbours, then
//! fall if it can, else spread sideways towards the nearest way down.

use super::rules::{Cell, UpdateRules};
use super::{BlockAccess, BlockChange, BlockUpdates, WAIT_TICKS, cell};
use crate::content::{BlockId, Face, Fluid, FluidLevel};
use crate::coords::BlockPos;
use std::collections::VecDeque;
use std::sync::Arc;

const SIDES: [Face; 4] = [Face::North, Face::South, Face::East, Face::West];

/// What a liquid block does this step.
enum Step {
    /// A neighbour is not loaded: try again later.
    Wait,
    Become(BlockId),
    Flow(Vec<(BlockPos, BlockId)>),
}

impl BlockUpdates {
    pub(super) fn update_fluid(
        &mut self,
        world: &mut impl BlockAccess,
        pos: BlockPos,
        changes: &mut Vec<BlockChange>,
    ) {
        let rules = Arc::clone(&self.rules);
        let Cell::Block(block) = cell(world, pos) else {
            return;
        };
        let Some(state) = rules.fluid_state(block) else {
            return;
        };
        let Some(fluid) = rules.fluid(state.source) else {
            return;
        };
        let view = View {
            rules: &rules,
            fluid,
            world: &*world,
        };
        match view.step(pos, state.level) {
            Step::Wait => self.plan(pos, self.now + WAIT_TICKS),
            Step::Become(next) => {
                self.set_block(world, pos, next, changes);
            }
            Step::Flow(targets) => {
                for (target, next) in targets {
                    self.set_block(world, target, next, changes);
                }
            }
        }
    }
}

struct View<'a, W> {
    rules: &'a UpdateRules,
    fluid: &'a Fluid,
    world: &'a W,
}

impl<W: BlockAccess> View<'_, W> {
    fn cell(&self, pos: BlockPos) -> Cell {
        cell(self.world, pos)
    }

    fn level(&self, cell: Cell) -> Option<FluidLevel> {
        self.rules.level_of(cell, self.fluid.source)
    }

    fn step(&self, pos: BlockPos, level: FluidLevel) -> Step {
        if let Some(hardened) = self.hardening(pos, level) {
            return Step::Become(hardened);
        }
        if level != FluidLevel::Source {
            let Some(wanted) = self.wanted_level(pos) else {
                return Step::Wait;
            };
            if wanted != Some(level) {
                let next = wanted.and_then(|level| self.fluid.block(level));
                return Step::Become(next.unwrap_or(BlockId::AIR));
            }
        }
        self.flow(pos, level)
    }

    /// Lava touching water (above or beside) turns into obsidian or stone.
    fn hardening(&self, pos: BlockPos, level: FluidLevel) -> Option<BlockId> {
        let hardens = self.fluid.hardens?;
        let touches = [Face::Up].into_iter().chain(SIDES).any(|face| {
            let cell = self.cell(pos.neighbour(face));
            self.rules.level_of(cell, hardens.touching).is_some()
        });
        touches.then_some(match level {
            FluidLevel::Source => hardens.source,
            _ => hardens.flowing,
        })
    }

    /// Level a flowing block should have given its neighbours; `Some(None)` = dries up,
    /// `None` = a neighbour is not loaded.
    fn wanted_level(&self, pos: BlockPos) -> Option<Option<FluidLevel>> {
        let above = self.cell(pos.neighbour(Face::Up));
        if above == Cell::Unloaded {
            return None;
        }
        if self.level(above).is_some() {
            return Some(Some(FluidLevel::Falling));
        }
        let mut nearest = u8::MAX;
        let mut sources = 0;
        for face in SIDES {
            let side = self.cell(pos.neighbour(face));
            if side == Cell::Unloaded {
                return None;
            }
            let fed = match self.level(side) {
                None => continue,
                Some(FluidLevel::Source) => {
                    sources += 1;
                    1
                }
                Some(FluidLevel::Falling) => 1,
                Some(FluidLevel::Flowing(level)) => level.saturating_add(1),
            };
            nearest = nearest.min(fed);
        }
        if self.fluid.renewable && sources >= 2 {
            let below = self.cell(pos.neighbour(Face::Down));
            let held =
                self.rules.cell_is_solid(below)? || self.level(below) == Some(FluidLevel::Source);
            if held {
                return Some(Some(FluidLevel::Source));
            }
        }
        Some((nearest <= self.fluid.spread).then_some(FluidLevel::Flowing(nearest)))
    }

    /// Falls if it can; otherwise spreads one level weaker to the sides that lead to the
    /// nearest way down (all open sides when none is in sight).
    fn flow(&self, pos: BlockPos, level: FluidLevel) -> Step {
        let down = pos.neighbour(Face::Down);
        let below = self.cell(down);
        if below == Cell::Unloaded {
            return Step::Wait;
        }
        if let Some(hardens) = self.fluid.hardens
            && self.rules.level_of(below, hardens.touching).is_some()
        {
            return Step::Flow(vec![(down, hardens.flowing)]);
        }
        if self.can_enter(below, FluidLevel::Falling) {
            return Step::Flow(vec![(down, self.fluid.falling)]);
        }
        let next = match level {
            FluidLevel::Source | FluidLevel::Falling => 1,
            FluidLevel::Flowing(level) => level.saturating_add(1),
        };
        let lands_in_itself = self.level(below).is_some();
        if (level != FluidLevel::Source && lands_in_itself) || next > self.fluid.spread {
            return Step::Flow(Vec::new());
        }
        let Some(block) = self.fluid.block(FluidLevel::Flowing(next)) else {
            return Step::Flow(Vec::new());
        };
        match self.downhill_sides(pos, FluidLevel::Flowing(next)) {
            None => Step::Wait,
            Some(sides) => Step::Flow(sides.into_iter().map(|side| (side, block)).collect()),
        }
    }

    fn downhill_sides(&self, pos: BlockPos, level: FluidLevel) -> Option<Vec<BlockPos>> {
        let mut nearest = u8::MAX;
        let mut sides = Vec::with_capacity(4);
        for face in SIDES {
            let side = pos.neighbour(face);
            let target = self.cell(side);
            if target == Cell::Unloaded {
                return None;
            }
            if !self.can_enter(target, level) {
                continue;
            }
            let distance = self.slope_distance(pos, side);
            if distance < nearest {
                nearest = distance;
                sides.clear();
            }
            if distance == nearest {
                sides.push(side);
            }
        }
        Some(sides)
    }

    /// Steps from `start` to the nearest place where the liquid could fall, searching
    /// `slope_search` blocks at most; `u8::MAX` when there is none.
    fn slope_distance(&self, from: BlockPos, start: BlockPos) -> u8 {
        let mut seen = vec![from, start];
        let mut frontier = VecDeque::from([(start, 0u8)]);
        while let Some((pos, distance)) = frontier.pop_front() {
            if self.is_hole(pos) {
                return distance;
            }
            if distance >= self.fluid.slope_search {
                continue;
            }
            for face in SIDES {
                let next = pos.neighbour(face);
                if !seen.contains(&next) && self.is_passable(self.cell(next)) {
                    seen.push(next);
                    frontier.push_back((next, distance + 1));
                }
            }
        }
        u8::MAX
    }

    fn is_hole(&self, pos: BlockPos) -> bool {
        let below = self.cell(pos.neighbour(Face::Down));
        self.rules.is_open(below) || self.level(below).is_some()
    }

    fn is_passable(&self, cell: Cell) -> bool {
        self.rules.is_open(cell)
            || matches!(self.level(cell), Some(level) if level != FluidLevel::Source)
    }

    /// Air and plants take any level; the same liquid is replaced only by a fuller one.
    fn can_enter(&self, cell: Cell, level: FluidLevel) -> bool {
        if self.rules.is_open(cell) {
            return true;
        }
        match (self.level(cell), level) {
            (Some(FluidLevel::Flowing(_)), FluidLevel::Falling) => true,
            (Some(FluidLevel::Flowing(old)), FluidLevel::Flowing(new)) => old > new,
            _ => false,
        }
    }
}
