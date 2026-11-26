//! The authoritative game state: players (ECS entities) and the blocks around them.

mod actions;
mod anvil;
mod bag_world;
mod bags;
mod bench;
mod boat;
mod bucket;
mod consume;
mod containers;
mod crafting;
mod damage;
mod drops;
mod equipment;
mod furnaces;
mod hammer;
mod inventory;
mod kits;
mod magnets;
mod mining;
mod placing;
mod player;
mod policy;
mod storage;
mod temperature;
mod terrain;
mod vitals;
mod vitals_save;
mod vitals_view;
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
use strata_core::coords::BlockPos;
use strata_core::dimension::DimensionId;
use strata_core::inventory::{DurabilityMode, PlayerInventory};
use strata_core::physics::{Body, MovementSettings};
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
    /// Chests and barrels with a magnet upgrade.
    magnets: magnets::Magnets,
    /// The strongest magnet among the player's bags, until the inventory changes.
    bag_magnet: Option<Option<(f32, u16)>>,
    /// World options of the survival gauges.
    survival_options: strata_core::survival::SurvivalOptions,
    /// Where the player wakes after fainting.
    spawn: DVec3,
    /// Biome names by `BiomeId` (salt water).
    biomes: Vec<strata_core::id::Identifier>,
    /// The gauges as the client last knew them.
    vitals_sent: Option<strata_protocol::VitalsView>,
    /// 0 at night, 1 by day (temperature).
    daylight: f32,
    /// The dimension simulated (temperature shifts by dimension).
    dimension: Option<strata_core::id::Identifier>,
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
        let spawn_point = spawn.position;
        let mut game = Self {
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
            magnets: Default::default(),
            bag_magnet: None,
            survival_options: Default::default(),
            spawn: spawn_point,
            biomes: Vec::new(),
            vitals_sent: None,
            daylight: 1.0,
            dimension: None,
        };
        game.fresh_vitals();
        game
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

    pub fn set_fluid_settings(&mut self, settings: FluidSettings) {
        self.updates.set_settings(settings);
    }

    pub fn set_fire_settings(&mut self, settings: FireSettings) {
        self.updates.set_fire_settings(settings);
    }

    pub fn set_world_seed(&mut self, seed: u64, dimension: &DimensionId) {
        self.updates.set_seed(seed, dimension);
        self.dimension = Some(dimension.0.clone());
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
        self.magnets_tick(loader);
        let mut changes = self.run_queued_actions(loader);
        self.vitals_tick(loader);
        changes.extend(self.mining_tick(loader));
        changes.extend(self.furnaces_tick(loader));
        let arrived = loader.take_arrived();
        for &pos in &arrived {
            self.wake_column_furnaces(pos);
            if let Some(column) = loader.column(pos) {
                self.updates.wake_fires(column);
            }
        }
        let mut world = Bounded { loader, border };
        let ticked = changes.len();
        self.updates.tick(&mut world, &mut changes);
        self.forget_heat(&changes, !arrived.is_empty());
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

#[cfg(test)]
mod storage_tests;

#[cfg(test)]
mod storage_rule_tests;

#[cfg(test)]
mod bag_tests;

#[cfg(test)]
mod bag_upkeep_tests;

#[cfg(test)]
mod vitals_tests;

#[cfg(test)]
mod consume_tests;

#[cfg(test)]
mod temperature_tests;
