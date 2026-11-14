//! Furnaces: only those burning or cooling are stepped each tick, so idle ones cost
//! nothing; a furnace in an unloaded column waits there until it comes back. The lit look
//! follows the fire, and a kit upgrades a furnace in place.

use super::Game;
use super::containers::block_centre;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::content::BlockId;
use strata_core::cooking::{FURNACE_MODULES, FurnaceState, FurnaceStats, ModuleDef, step};
use strata_core::coords::BlockPos;
use strata_core::inventory::Container;
use strata_core::updates::BlockChange;
use strata_protocol::commands::Feedback;
use strata_protocol::{FurnaceView, ServerMessage};

/// Ticks of a usual cooking (10 s) and of a piece of coal, for the screen's figures.
const USUAL_COOKING: u32 = 200;
const COAL: u32 = 1600;

impl Game {
    /// Something may have changed in the furnace at `pos`: it is stepped from now on.
    pub(super) fn wake_furnace(&mut self, pos: BlockPos) {
        self.furnaces.insert(pos);
        self.furnace_view = None;
    }

    #[cfg(test)]
    pub(crate) fn furnace_is_active(&self, pos: BlockPos) -> bool {
        self.furnaces.contains(&pos)
    }

    pub(super) fn furnaces_tick(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        let mut changes = Vec::new();
        let mut active: Vec<BlockPos> = self.furnaces.iter().copied().collect();
        active.sort_by_key(|pos| (pos.x, pos.y, pos.z));
        for pos in active {
            if let Some(false) = self.step_furnace(loader, pos, &mut changes) {
                self.furnaces.remove(&pos);
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
        let Some(stats) = self.furnace_stats(block, pos) else {
            return Some(false);
        };
        let open = self.open_furnace() == Some(pos);
        let rules = &self.rules;
        let Some(entity) = self.block_entities.get_mut(&pos) else {
            return Some(false);
        };
        let state = entity.furnace.get_or_insert_with(FurnaceState::default);
        let was_lit = state.is_lit();
        let done = step(state, &mut entity.container, &stats, &rules.cooking, rules);
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
        let empty = Container::default();
        let slots = self
            .block_entities
            .get(&pos)
            .map_or(&empty, |entity| &entity.container);
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

    fn send_furnace_view(&mut self) {
        let Some(view) = self
            .open_furnace()
            .and_then(|pos| self.furnace_view_at(pos))
        else {
            return;
        };
        if self.furnace_view != Some(view) {
            self.furnace_view = Some(view);
            self.messages.push(ServerMessage::Furnace(view));
        }
    }

    fn furnace_view_at(&self, pos: BlockPos) -> Option<FurnaceView> {
        let window = self.ecs.windows.get(self.player)?;
        let stats = self.furnace_stats(window.block, pos)?;
        let entity = self.block_entities.get(&pos)?;
        let state = entity.furnace.clone().unwrap_or_default();
        let recipe = entity
            .container
            .get(strata_core::cooking::FURNACE_INPUT)
            .and_then(|input| self.rules.cooking.recipe_for(input.item, &stats.kinds));
        let usual = (f64::from(USUAL_COOKING) * stats.time_factor).max(1.0);
        Some(FurnaceView {
            burn_left: state.burn_left,
            burn_total: state.burn_total,
            progress: state.progress,
            cook_time: recipe.map_or(0, |r| stats.cook_ticks(r)),
            seconds: (usual / 20.0) as f32,
            per_coal: (f64::from(stats.burn_ticks(COAL)) / usual) as f32,
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
