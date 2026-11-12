//! The authoritative game state: players (ECS entities) and the blocks around them.

mod actions;
mod boat;
mod bucket;
mod terrain;

use crate::ecs::{Ecs, Entity, PlayerControl};
use crate::loader::{Bounded, ColumnLoader};
use crate::rules::BlockRules;
use crate::systems::{boat_movement, player_movement, wake_boats};
use glam::DVec3;
use std::sync::Arc;
use strata_core::content::{BlockId, Face};
use strata_core::coords::{BlockPos, WORLD_HEIGHT};
use strata_core::dimension::DimensionId;
use strata_core::physics::{Aabb, Body, MoveInput, MovementSettings};
pub use strata_core::updates::BlockChange;
use strata_core::updates::{BlockUpdates, FireSettings, FluidSettings, cell};
use strata_protocol::{GameMode, HotbarState, ServerMessage};

/// Farthest a player may break or place, measured from the eyes to the block centre
/// (a little more than the client's reach, for rounding).
const MAX_REACH: f64 = 6.0;
const MAX_QUEUED_INPUTS: usize = 40;
/// Ticks between two block actions of one player (10 per second at most).
const ACTION_TICKS: u64 = 2;
const HOTBAR_SLOTS: usize = 9;

pub struct Game {
    pub ecs: Ecs,
    pub player: Entity,
    pub rules: BlockRules,
    /// Half-width of the world, in blocks.
    pub border: i32,
    updates: BlockUpdates,
    tick: u64,
    /// News for the clients (entities, action answers), taken by the server after each
    /// tick or message, after the block changes.
    messages: Vec<ServerMessage>,
}

impl Game {
    pub fn new(rules: BlockRules, border: i32, spawn: Body, mode: GameMode) -> Self {
        let mut ecs = Ecs::default();
        let player = ecs.entities.spawn();
        ecs.bodies.insert(player, spawn);
        ecs.hotbars.insert(player, HotbarState::default());
        let movement = MovementSettings {
            can_fly: mode == GameMode::Creative,
            step_up: true,
            auto_jump: true,
        };
        ecs.controls
            .insert(player, PlayerControl::new(movement, mode));
        let updates = BlockUpdates::new(Arc::clone(rules.updates()), FluidSettings::default());
        Self {
            ecs,
            player,
            rules,
            border,
            updates,
            tick: 0,
            messages: Vec::new(),
        }
    }

    /// The player left: their entity and all its components go away.
    pub fn remove_player(&mut self) {
        self.ecs.despawn(self.player);
    }

    pub fn player_body(&self) -> Option<&Body> {
        self.ecs.bodies.get(self.player)
    }

    pub fn movement(&self) -> Option<MovementSettings> {
        self.ecs.controls.get(self.player).map(|c| c.movement)
    }

    pub fn queue_input(&mut self, sequence: u32, input: MoveInput) {
        let valid = input.yaw.is_finite()
            && input.pitch.is_finite()
            && input.wish.iter().all(|w| w.is_finite());
        if let (true, Some(control)) = (valid, self.ecs.controls.get_mut(self.player))
            && control.pending.len() < MAX_QUEUED_INPUTS
        {
            control.pending.push_back((sequence, input));
        }
    }

    /// Creative only: the player flies at the destination, kept inside the border and the
    /// world height.
    pub fn teleport(&mut self, feet: [f64; 3]) -> bool {
        let border = f64::from(self.border) - 1.0;
        if !feet.iter().all(|c| c.is_finite()) || !self.is_creative() {
            return false;
        }
        self.leave_boat(self.player);
        let Some(body) = self.ecs.bodies.get_mut(self.player) else {
            return false;
        };
        {
            body.position = DVec3::new(
                feet[0].clamp(-border, border),
                feet[1].clamp(0.0, f64::from(WORLD_HEIGHT)),
                feet[2].clamp(-border, border),
            );
            body.velocity = DVec3::ZERO;
            body.flying = true;
        }
        true
    }

    /// Creative: any existing item, nine slots at most. Returns whether it was accepted.
    pub fn set_hotbar(&mut self, hotbar: HotbarState) -> bool {
        let valid = hotbar.slots.len() <= HOTBAR_SLOTS
            && usize::from(hotbar.selected) < HOTBAR_SLOTS
            && hotbar
                .slots
                .iter()
                .flatten()
                .all(|&item| self.rules.is_item(item))
            && self.is_creative();
        if valid {
            self.ecs.hotbars.insert(self.player, hotbar);
        }
        valid
    }

    pub fn look(&self) -> [f32; 2] {
        self.ecs
            .controls
            .get(self.player)
            .map_or([0.0; 2], |c| c.look)
    }

    pub fn set_look(&mut self, look: [f32; 2]) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.look = look;
        }
    }

    pub fn hotbar(&self) -> HotbarState {
        self.ecs
            .hotbars
            .get(self.player)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_preferences(&mut self, auto_jump: bool, step_up: bool) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.movement.auto_jump = auto_jump;
            control.movement.step_up = step_up;
        }
    }

    pub fn set_fluid_settings(&mut self, settings: FluidSettings) {
        self.updates.set_settings(settings);
    }

    pub fn set_fire_settings(&mut self, settings: FireSettings) {
        self.updates.set_fire_settings(settings);
    }

    pub fn set_world_seed(&mut self, seed: u64, dimension: &DimensionId) {
        self.updates.set_seed(seed, dimension);
    }

    /// Runs one tick of player and boat physics, liquids and fire; returns the blocks changed.
    pub fn tick(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        self.tick += 1;
        let border = self.border;
        let terrain = terrain::WorldTerrain {
            loader,
            rules: &self.rules,
            border,
        };
        let dismounts = player_movement(&mut self.ecs, &terrain);
        for rider in dismounts {
            self.dismount(loader, rider);
        }
        let terrain = terrain::WorldTerrain {
            loader,
            rules: &self.rules,
            border,
        };
        let moved = boat_movement(&mut self.ecs, &self.rules, &terrain);
        self.report_moves(&moved);
        let mut changes = self.run_queued_action(loader);
        for pos in loader.take_arrived() {
            if let Some(column) = loader.column(pos) {
                self.updates.wake_fires(column);
            }
        }
        let mut world = Bounded { loader, border };
        self.updates.tick(&mut world, &mut changes);
        wake_boats(&mut self.ecs, &self.rules, &changes);
        changes
    }

    pub fn last_sequence(&self) -> u32 {
        self.ecs
            .controls
            .get(self.player)
            .map_or(0, |c| c.last_sequence)
    }

    /// Validated break: within reach, an actual block the player may break. The changes
    /// also hold the neighbours' reactions (a plant losing its ground); empty when refused.
    pub fn break_block(&mut self, loader: &mut ColumnLoader, pos: BlockPos) -> Vec<BlockChange> {
        if !pos.is_valid(self.border) {
            return Vec::new();
        }
        let Some(current) = loader.block(pos) else {
            return Vec::new();
        };
        let allowed = current != BlockId::AIR
            && (self.is_creative() || self.rules.is_breakable(current))
            && self.in_reach(pos)
            && self.take_action();
        let mut changes = Vec::new();
        if allowed {
            self.set_block(loader, pos, BlockId::AIR, &mut changes);
        }
        changes
    }

    fn set_block(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
        block: BlockId,
        changes: &mut Vec<BlockChange>,
    ) {
        let border = self.border;
        let mut world = Bounded { loader, border };
        self.updates.set_block(&mut world, pos, block, changes);
        wake_boats(&mut self.ecs, &self.rules, changes);
    }

    /// Validated placement into a free (non-solid) cell that no player stands in: next to
    /// a solid block, or in place of a replaceable one (plants). Turned blocks (logs)
    /// follow the clicked face; plants need their support (ground, or a ceiling for vines).
    pub fn place_block(
        &mut self,
        loader: &mut ColumnLoader,
        against: BlockPos,
        face: Face,
        block: BlockId,
    ) -> Vec<BlockChange> {
        let Some(target) = self.placement_target(loader, against, face) else {
            return Vec::new();
        };
        let world = Bounded {
            loader,
            border: self.border,
        };
        let supported = self.rules.updates().is_supported(
            block,
            cell(&world, target.neighbour(Face::Down)),
            cell(&world, target.neighbour(Face::Up)),
        );
        let allowed = self.rules.is_placeable(block)
            && supported == Some(true)
            && self.in_reach(target)
            && !self.blocks_a_player(target, block)
            && self.take_action();
        let block = self.rules.placed_against(block, face);
        let mut changes = Vec::new();
        if allowed {
            self.set_block(loader, target, block, &mut changes);
        }
        changes
    }

    /// The cell a block placed against `against` goes to, if any.
    fn placement_target(
        &self,
        loader: &ColumnLoader,
        against: BlockPos,
        face: Face,
    ) -> Option<BlockPos> {
        if !against.is_valid(self.border) {
            return None;
        }
        let aimed = loader.block(against)?;
        if self.rules.is_replaceable(aimed) {
            return Some(against);
        }
        let target = against.neighbour(face);
        let current = loader.block(target)?;
        (self.rules.is_solid(aimed)
            && !self.rules.is_solid(current)
            && target.is_valid(self.border))
        .then_some(target)
    }

    pub fn set_mode(&mut self, mode: GameMode) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.mode = mode;
            control.movement.can_fly = mode == GameMode::Creative;
        }
    }

    pub fn is_creative(&self) -> bool {
        self.ecs
            .controls
            .get(self.player)
            .is_some_and(|c| c.mode == GameMode::Creative)
    }

    /// Rate limit shared by breaking and placing; consumes the action when allowed.
    fn take_action(&mut self) -> bool {
        let tick = self.tick;
        self.ecs.controls.get_mut(self.player).is_some_and(|c| {
            let ready = c.last_action.is_none_or(|last| tick >= last + ACTION_TICKS);
            if ready {
                c.last_action = Some(tick);
            }
            ready
        })
    }

    fn in_reach(&self, pos: BlockPos) -> bool {
        self.player_body().is_some_and(|body| {
            let center = DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)) + 0.5;
            body.eyes().distance(center) <= MAX_REACH
        })
    }

    fn blocks_a_player(&self, pos: BlockPos, block: BlockId) -> bool {
        if !self.rules.is_solid(block) {
            return false;
        }
        let min = DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z));
        let cell = Aabb {
            min,
            max: min + 1.0,
        };
        self.ecs
            .bodies
            .iter()
            .any(|(_, body)| body.aabb().overlaps(&cell))
    }
}

#[cfg(test)]
mod tests;
