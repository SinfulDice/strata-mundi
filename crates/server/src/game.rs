//! The authoritative game state: players (ECS entities) and the blocks around them.

mod actions;
mod anvil;
mod bench;
mod boat;
mod bucket;
mod containers;
mod crafting;
mod drops;
mod equipment;
mod furnaces;
mod hammer;
mod inventory;
mod mining;
mod placing;
mod terrain;
mod work;

pub use containers::{BlockEntities, BlockEntity, OpenWindow};
pub use crafting::CraftWindow;

use crate::ecs::{Ecs, Entity, PlayerControl};
use crate::loader::{Bounded, ColumnLoader};
use crate::rules::BlockRules;
use crate::systems::{boat_movement, player_movement, wake_boats};
use glam::DVec3;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, WORLD_HEIGHT};
use strata_core::dimension::DimensionId;
use strata_core::inventory::{DurabilityMode, PlayerInventory};
use strata_core::physics::{Body, MoveInput, MovementSettings};
pub use strata_core::updates::BlockChange;
use strata_core::updates::{BlockUpdates, FireSettings, FluidSettings};
use strata_protocol::{FurnaceView, GameMode, MIN_VIEW_DISTANCE, ServerMessage};

/// Farthest a player may break or place, measured from the eyes to the block centre
/// (a little more than the client's reach, for rounding).
const MAX_REACH: f64 = 6.0;
const MAX_QUEUED_INPUTS: usize = 40;
/// Ticks between two block actions of one player (10 per second at most).
const ACTION_TICKS: u64 = 2;

pub struct Game {
    pub ecs: Ecs,
    pub player: Entity,
    pub rules: BlockRules,
    /// Half-width of the world, in blocks.
    pub border: i32,
    /// Columns the player's client has around them (refusals tell only about those).
    view_distance: u32,
    updates: BlockUpdates,
    tick: u64,
    /// News for the clients (entities, action answers), taken by the server after each
    /// tick or message, after the block changes.
    messages: Vec<ServerMessage>,
    /// Contents of the container blocks of the dimension.
    pub block_entities: BlockEntities,
    /// The inventory changed since the last messages were taken.
    inventory_dirty: bool,
    /// Mixed with the block and the tick for each loot roll.
    loot_seed: u64,
    /// The crafting grid changed since the last messages were taken.
    craft_dirty: bool,
    /// World option: recipes are learnt by playing.
    discover_recipes: bool,
    /// World option: what becomes of worn-out items.
    pub durability: DurabilityMode,
    /// Recipes learnt whose pack is gone, saved again as they were.
    pub unknown_recipes: Vec<strata_core::id::Identifier>,
    /// Furnaces burning or cooling: only these are stepped each tick, with their speed
    /// (computed again when `None`).
    furnaces: BTreeMap<BlockPos, Option<strata_core::cooking::FurnaceStats>>,
    /// Active furnaces whose column unloaded, woken when it comes back.
    waiting_furnaces: HashMap<strata_core::coords::ColumnPos, Vec<BlockPos>>,
    /// Fire and cooking of the open furnace, as the client last knew them.
    furnace_view: Option<FurnaceView>,
    /// Stats of the hand and the armour, upgrades counted.
    stats_cache: strata_core::upgrade::StatsCache,
    /// What is worn, until a worn slot changes.
    worn_cache: strata_core::worn::WornCache,
}

impl Game {
    pub fn new(rules: BlockRules, border: i32, spawn: Body, mode: GameMode) -> Self {
        let mut ecs = Ecs::default();
        let player = ecs.entities.spawn();
        ecs.bodies.insert(player, spawn);
        ecs.inventories.insert(player, PlayerInventory::default());
        ecs.crafts.insert(player, CraftWindow::default());
        ecs.recipes
            .insert(player, rules.recipes.starting().collect());
        let movement = MovementSettings {
            can_fly: mode == GameMode::Creative,
            ..MovementSettings::default()
        };
        ecs.controls
            .insert(player, PlayerControl::new(movement, mode));
        let updates = BlockUpdates::new(Arc::clone(rules.updates()), FluidSettings::default());
        Self {
            ecs,
            player,
            rules,
            border,
            view_distance: MIN_VIEW_DISTANCE,
            updates,
            tick: 0,
            messages: Vec::new(),
            block_entities: BlockEntities::default(),
            inventory_dirty: false,
            loot_seed: 0,
            craft_dirty: false,
            discover_recipes: true,
            durability: DurabilityMode::default(),
            unknown_recipes: Vec::new(),
            furnaces: BTreeMap::new(),
            waiting_furnaces: HashMap::new(),
            stats_cache: Default::default(),
            worn_cache: Default::default(),
            furnace_view: None,
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
        let dimension = strata_core::hash::fnv1a(dimension.0.to_string().as_bytes());
        self.loot_seed = seed ^ dimension.rotate_left(17);
    }

    /// Runs one tick of player and boat physics, liquids and fire; returns the blocks changed.
    pub fn tick(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        self.tick += 1;
        self.apply_equipment();
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
        self.drops_tick(loader);
        let mut changes = self.run_queued_actions(loader);
        changes.extend(self.mining_tick(loader));
        changes.extend(self.furnaces_tick(loader));
        for pos in loader.take_arrived() {
            self.wake_column_furnaces(pos);
            if let Some(column) = loader.column(pos) {
                self.updates.wake_fires(column);
            }
        }
        let mut world = Bounded { loader, border };
        let ticked = changes.len();
        self.updates.tick(&mut world, &mut changes);
        wake_boats(&mut self.ecs, &self.rules, &changes);
        self.settle_block_entities(loader, &changes[ticked..]);
        changes
    }

    pub fn last_sequence(&self) -> u32 {
        self.ecs
            .controls
            .get(self.player)
            .map_or(0, |c| c.last_sequence)
    }

    /// Validated break: within reach, an actual block the player may break (in survival,
    /// hit long enough since `StartBreak`). The changes
    /// also hold the neighbours' reactions (a plant losing its ground); empty when refused.
    pub fn break_block(&mut self, loader: &mut ColumnLoader, pos: BlockPos) -> Vec<BlockChange> {
        if !pos.is_valid(self.border) {
            return Vec::new();
        }
        let Some(current) = loader.block(pos) else {
            return Vec::new();
        };
        let allowed = current != BlockId::AIR
            && (self.is_creative()
                || (self.rules.is_breakable(current) && self.hit_long_enough(pos, current)))
            && self.in_reach(pos)
            && self.take_action();
        if !allowed {
            return Vec::new();
        }
        self.finish_break(loader, pos, current)
    }

    pub(super) fn set_block(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
        block: BlockId,
        changes: &mut Vec<BlockChange>,
    ) {
        let border = self.border;
        let before = changes.len();
        let mut world = Bounded { loader, border };
        self.updates.set_block(&mut world, pos, block, changes);
        wake_boats(&mut self.ecs, &self.rules, changes);
        self.settle_block_entities(loader, &changes[before..]);
    }

    pub fn set_mode(&mut self, mode: GameMode) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.mode = mode;
            control.movement.can_fly = mode == GameMode::Creative;
            self.messages.push(ServerMessage::Mode(mode));
            self.recipes_changed();
            if mode != GameMode::Creative
                && let Some(body) = self.ecs.bodies.get_mut(self.player)
            {
                body.flying = false;
            }
        }
    }

    pub fn mode(&self) -> GameMode {
        self.ecs
            .controls
            .get(self.player)
            .map_or(GameMode::Creative, |c| c.mode)
    }

    pub fn set_view_distance(&mut self, distance: u32) {
        self.view_distance = distance;
    }

    pub fn is_creative(&self) -> bool {
        self.ecs
            .controls
            .get(self.player)
            .is_some_and(|c| c.mode == GameMode::Creative)
    }

    /// Rate limit shared by breaking and placing; consumes the action when allowed.
    pub(super) fn take_action(&mut self) -> bool {
        let tick = self.tick;
        self.ecs.controls.get_mut(self.player).is_some_and(|c| {
            let ready = c.last_action.is_none_or(|last| tick >= last + ACTION_TICKS);
            if ready {
                c.last_action = Some(tick);
            }
            ready
        })
    }

    pub(super) fn in_reach(&self, pos: BlockPos) -> bool {
        self.player_body().is_some_and(|body| {
            let center = DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)) + 0.5;
            body.eyes().distance(center) <= MAX_REACH
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod inventory_tests;

#[cfg(test)]
mod conservation_tests;

#[cfg(test)]
mod mining_tests;

#[cfg(test)]
mod crafting_tests;

#[cfg(test)]
mod furnace_tests;

#[cfg(test)]
mod gear_tests;

#[cfg(test)]
mod anvil_tests;

#[cfg(test)]
mod hammer_tests;

#[cfg(test)]
mod equipment_tests;

#[cfg(test)]
mod mobility_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod input_order_tests;
