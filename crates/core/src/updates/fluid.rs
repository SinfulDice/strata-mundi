//! One step of a liquid block, close to Minecraft's: fill level from the neighbours, then
//! fall if it can, else spread sideways towards the nearest way down.

use super::rules::{Cell, UpdateRules};
use super::{BlockAccess, BlockChange, BlockUpdates, WAIT_TICKS, cell};
use crate::content::MAX_SLOPE_SEARCH;
use crate::content::{BlockId, Face, Fluid, FluidLevel};
use crate::coords::BlockPos;

const SIDES: [Face; 4] = [Face::North, Face::South, Face::East, Face::West];
/// Width of the square a slope search can cover around its start.
const SEARCH_WIDTH: usize = 2 * MAX_SLOPE_SEARCH as usize + 1;

/// What a liquid block does this step.
enum Step {
    /// A neighbour is not loaded: try again later.
    Wait,
    Become(BlockId),
    Flow(Targets),
}

/// Up to four blocks to write, without allocating.
struct Targets {
    items: [(BlockPos, BlockId); 4],
    len: usize,
}

impl Default for Targets {
    fn default() -> Self {
        Self {
            items: [(BlockPos::new(0, 0, 0), BlockId::AIR); 4],
            len: 0,
        }
    }
}

impl Targets {
    fn one(pos: BlockPos, block: BlockId) -> Self {
        let mut targets = Self::default();
        targets.push(pos, block);
        targets
    }

    fn push(&mut self, pos: BlockPos, block: BlockId) {
        if let Some(slot) = self.items.get_mut(self.len) {
            *slot = (pos, block);
            self.len += 1;
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn as_slice(&self) -> &[(BlockPos, BlockId)] {
        &self.items[..self.len]
    }
}

impl BlockUpdates {
    pub(super) fn update_fluid(
        &mut self,
        world: &mut impl BlockAccess,
        pos: BlockPos,
        changes: &mut Vec<BlockChange>,
    ) {
        let Some(step) = self.step(world, pos) else {
            return;
        };
        match step {
            Step::Wait => self.plan(pos, self.now + WAIT_TICKS),
            Step::Become(next) => {
                self.set_block(world, pos, next, changes);
            }
            Step::Flow(targets) => {
                for &(target, next) in targets.as_slice() {
                    self.set_block(world, target, next, changes);
                }
            }
        }
    }

    fn step(&self, world: &impl BlockAccess, pos: BlockPos) -> Option<Step> {
        let Cell::Block(block) = cell(world, pos) else {
            return None;
        };
        let state = self.rules.fluid_state(block)?;
        let view = View {
            rules: &self.rules,
            fluid: self.rules.fluid(state.source)?,
            world,
        };
        Some(view.step(pos, state.level))
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
            return Step::Flow(Targets::one(down, hardens.flowing));
        }
        if self.can_enter(below, FluidLevel::Falling) {
            return Step::Flow(Targets::one(down, self.fluid.falling));
        }
        let next = match level {
            FluidLevel::Source | FluidLevel::Falling => 1,
            FluidLevel::Flowing(level) => level.saturating_add(1),
        };
        let lands_in_itself = self.level(below).is_some();
        if (level != FluidLevel::Source && lands_in_itself) || next > self.fluid.spread {
            return Step::Flow(Targets::default());
        }
        let Some(block) = self.fluid.block(FluidLevel::Flowing(next)) else {
            return Step::Flow(Targets::default());
        };
        match self.downhill_sides(pos, FluidLevel::Flowing(next), block) {
            None => Step::Wait,
            Some(sides) => Step::Flow(sides),
        }
    }

    fn downhill_sides(&self, pos: BlockPos, level: FluidLevel, block: BlockId) -> Option<Targets> {
        let mut nearest = u8::MAX;
        let mut sides = Targets::default();
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
                sides.push(side, block);
            }
        }
        Some(sides)
    }

    /// Steps from `start` to the nearest place where the liquid could fall, searching
    /// `slope_search` blocks at most; `u8::MAX` when there is none.
    /// Breadth-first on fixed arrays: every block is queued once, so the queue never
    /// holds more than the square around `start`.
    fn slope_distance(&self, from: BlockPos, start: BlockPos) -> u8 {
        let mut seen = [[false; SEARCH_WIDTH]; SEARCH_WIDTH];
        let mut queue = [(start, 0u8); SEARCH_WIDTH * SEARCH_WIDTH];
        let (mut head, mut tail) = (0, 1);
        mark(&mut seen, start, from);
        mark(&mut seen, start, start);
        while head < tail {
            let (pos, distance) = queue[head];
            head += 1;
            if self.is_hole(pos) {
                return distance;
            }
            if distance >= self.fluid.slope_search {
                continue;
            }
            for face in SIDES {
                let next = pos.neighbour(face);
                if self.is_passable(self.cell(next)) && mark(&mut seen, start, next) {
                    queue[tail] = (next, distance + 1);
                    tail += 1;
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

/// Marks `pos` as seen in the square centred on `start`; `false` if it already was (or
/// lies outside the square, which a search bounded by `MAX_SLOPE_SEARCH` never reaches).
fn mark(seen: &mut [[bool; SEARCH_WIDTH]; SEARCH_WIDTH], start: BlockPos, pos: BlockPos) -> bool {
    let centre = i32::from(MAX_SLOPE_SEARCH);
    let index = |a: i32, b: i32| usize::try_from(a - b + centre).ok();
    let cell = index(pos.x, start.x)
        .zip(index(pos.z, start.z))
        .and_then(|(x, z)| seen.get_mut(x)?.get_mut(z));
    match cell {
        Some(cell) if !*cell => {
            *cell = true;
            true
        }
        _ => false,
    }
}
