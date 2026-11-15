//! Furnaces: only those burning or cooling are stepped each tick, so idle ones cost
//! nothing; one whose column unloads waits, and is woken when the column comes back. The
//! lit look follows the fire, and a kit upgrades a furnace in place.

use super::Game;
use super::containers::block_centre;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::content::BlockId;
use strata_core::cooking::{
    DEFAULT_COOK_TIME, FURNACE_MODULES, Furnace, FurnaceState, FurnaceStats, ModuleDef, step,
};
use strata_core::coords::{BlockPos, ColumnPos};
use strata_core::updates::BlockChange;
use strata_protocol::commands::Feedback;
use strata_protocol::{FurnaceView, ServerMessage};

impl Game {
    /// Something may have changed in the furnace at `pos` (contents, modules, tier): it is
    /// stepped from now on, its speed computed again.
    pub(super) fn wake_furnace(&mut self, pos: BlockPos) {
        self.furnaces.insert(pos, None);
    }

    /// Furnaces left waiting in a column that came back go on.
    pub(super) fn wake_column_furnaces(&mut self, column: ColumnPos) {
        for pos in self.waiting_furnaces.remove(&column).unwrap_or_default() {
            self.wake_furnace(pos);
        }
    }

    #[cfg(test)]
    pub(crate) fn furnace_is_active(&self, pos: BlockPos) -> bool {
        self.furnaces.contains_key(&pos)
    }

    pub(super) fn furnaces_tick(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        let mut changes = Vec::new();
        let active: Vec<BlockPos> = self.furnaces.keys().copied().collect();
        for pos in active {
            match self.step_furnace(loader, pos, &mut changes) {
                Some(true) => {}
                Some(false) => {
                    self.furnaces.remove(&pos);
                }
                None => {
                    self.furnaces.remove(&pos);
                    self.waiting_furnaces
                        .entry(pos.column())
                        .or_default()
                        .push(pos);
                }
            }
        }
        self.send_furnace_view();
        changes
    }

    /// One tick of the furnace at `pos`: whether it is still active (`None` when its
    /// column is not loaded: it waits).
    fn step_furnace(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
        changes: &mut Vec<BlockChange>,
    ) -> Option<bool> {
        let block = loader.block(pos)?;
        if self.furnaces.get(&pos).is_some_and(Option::is_none) {
            let stats = self.furnace_stats(block, pos);
            self.furnaces.insert(pos, stats);
        }
        let open = self.open_furnace() == Some(pos);
        let rules = &self.rules;
        let (Some(def), Some(Some(stats)), Some(entity)) = (
            rules.furnace(block),
            self.furnaces.get(&pos),
            self.block_entities.get_mut(&pos),
        ) else {
            return Some(false);
        };
        let furnace = Furnace {
            stats,
            kinds: &def.kinds,
            cooking: &rules.cooking,
        };
        let state = entity.furnace.get_or_insert_with(FurnaceState::default);
        let was_lit = state.is_lit();
        let done = step(state, &mut entity.container, &furnace, rules);
        let lit = state.is_lit();
        if done.slots && open {
            let contents = Box::new(entity.container.clone());
            self.messages
                .push(ServerMessage::ContainerContents(contents));
        }
        if was_lit != lit {
            self.light_furnace(loader, pos, block, lit, changes);
        }
        Some(done.active)
    }

    /// Station, tier and the modules in the upgrade slots of the furnace at `pos`.
    fn furnace_stats(&self, block: BlockId, pos: BlockPos) -> Option<FurnaceStats> {
        let furnace = self.rules.furnace(block)?;
        let tier = self.rules.cooking.tier(&furnace.tier).copied()?;
        let slots = &self.block_entities.get(&pos)?.container;
        let modules: Vec<ModuleDef> = (FURNACE_MODULES..slots.len())
            .filter_map(|index| slots.get(index))
            .filter_map(|stack| self.rules.items().get(stack.item)?.module)
            .collect();
        Some(FurnaceStats::new(furnace, &tier, &modules))
    }

    fn light_furnace(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
        block: BlockId,
        lit: bool,
        changes: &mut Vec<BlockChange>,
    ) {
        let Some(variant) = self.rules.facing(block).map(|f| f.lit(lit)) else {
            return;
        };
        if variant != block {
            self.follow_open_block(pos, variant);
            self.set_block(loader, pos, variant, changes);
        }
    }

    /// The open screen keeps showing a furnace whose block changed (lit, upgraded).
    fn follow_open_block(&mut self, pos: BlockPos, block: BlockId) {
        if let Some(window) = self.ecs.windows.get_mut(self.player)
            && window.halves == [pos]
        {
            window.block = block;
        }
    }

    /// Where the furnace the player has open stands.
    fn open_furnace(&self) -> Option<BlockPos> {
        let window = self.ecs.windows.get(self.player)?;
        self.rules.furnace(window.block)?;
        window.halves.first().copied()
    }

    /// The open furnace's view goes out only when it leaves the course the client
    /// follows on its own (`FurnaceView::advanced`).
    fn send_furnace_view(&mut self) {
        let Some(view) = self.open_furnace().and_then(|pos| {
            let block = self.ecs.windows.get(self.player)?.block;
            self.furnace_view_at(block, pos)
        }) else {
            self.furnace_view = None;
            return;
        };
        let expected = self.furnace_view.map(FurnaceView::advanced);
        if expected != Some(view) {
            self.messages.push(ServerMessage::Furnace(view));
        }
        self.furnace_view = Some(view);
    }

    pub(super) fn furnace_view_at(&self, block: BlockId, pos: BlockPos) -> Option<FurnaceView> {
        let def = self.rules.furnace(block)?;
        let stats = match self.furnaces.get(&pos) {
            Some(Some(stats)) => stats.clone(),
            _ => self.furnace_stats(block, pos)?,
        };
        let entity = self.block_entities.get(&pos)?;
        let state = entity.furnace.clone().unwrap_or_default();
        let furnace = Furnace {
            stats: &stats,
            kinds: &def.kinds,
            cooking: &self.rules.cooking,
        };
        let usual = (f64::from(DEFAULT_COOK_TIME) * stats.time_factor).max(1.0);
        let per_fuel = |ticks: u32| (f64::from(stats.burn_ticks(ticks)) / usual) as f32;
        Some(FurnaceView {
            burn_left: state.burn_left,
            burn_total: state.burn_total,
            progress: state.progress,
            cook_time: furnace.cook_time(&entity.container, &self.rules),
            seconds: (usual / 20.0) as f32,
            per_coal: self.rules.cooking.reference_fuel().map(per_fuel),
        })
    }

    /// Right click on a block: a kit in hand upgrades the furnace it is used on;
    /// otherwise the block opens (container, station, furnace).
    pub(super) fn use_block(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
    ) -> Vec<BlockChange> {
        if let Some(changes) = self.upgrade_with_kit(loader, pos) {
            return changes;
        }
        self.open_block(loader, pos);
        Vec::new()
    }

    /// `None` when the hand holds no kit or the block is no furnace.
    fn upgrade_with_kit(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
    ) -> Option<Vec<BlockChange>> {
        let item = self.held_item()?;
        let kit = self.rules.items().get(item)?.kit.clone()?;
        let block = loader.block(pos)?;
        let tier = self.rules.furnace(block)?.tier.clone();
        let mut changes = Vec::new();
        if !self.in_reach(pos) || !self.take_action() {
            return Some(changes);
        }
        let upgraded = (tier == kit.from)
            .then(|| self.rules.furnace_of_tier(block, &kit.to))
            .flatten();
        let Some((upgraded, size)) = upgraded.and_then(|b| Some((b, self.rules.entity_slots(b)?)))
        else {
            let refused = Feedback::error("feedback-kit-wrong-tier", &[]);
            self.messages.push(ServerMessage::Feedback(refused));
            return Some(changes);
        };
        let entity = self.block_entities.entry(pos).or_default();
        entity.furnace.get_or_insert_with(FurnaceState::default);
        let cut = entity.container.resize(size);
        let contents = entity.container.clone();
        for stack in cut {
            self.spawn_drop(stack, block_centre(pos), DVec3::ZERO);
        }
        self.follow_open_block(pos, upgraded);
        self.set_block(loader, pos, upgraded, &mut changes);
        self.consume_held();
        self.wake_furnace(pos);
        if self.open_furnace() == Some(pos) {
            self.messages
                .push(ServerMessage::ContainerContents(Box::new(contents)));
        }
        let done = Feedback::ok("feedback-kit-done", &[]);
        self.messages.push(ServerMessage::Feedback(done));
        Some(changes)
    }
}
