//! The authoritative game state: players (ECS entities) and the blocks around them.

use crate::ecs::{Ecs, Entity, PlayerControl};
use crate::loader::{Bounded, ColumnLoader};
use crate::rules::BlockRules;
use crate::systems::player_movement;
use glam::DVec3;
use std::sync::Arc;
use strata_core::content::{BlockId, Face};
use strata_core::coords::{BlockPos, WORLD_HEIGHT};
use strata_core::physics::{Aabb, Body, MoveInput, MovementSettings, solid_at};
pub use strata_core::updates::BlockChange;
use strata_core::updates::{BlockUpdates, FluidSettings, cell};
use strata_protocol::{GameMode, HotbarState};

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

    /// Runs one tick of player physics and liquids; returns the blocks changed.
    pub fn tick(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        self.tick += 1;
        let (rules, border) = (&self.rules, self.border);
        let solid = |x: i32, y: i32, z: i32| {
            solid_at(BlockPos::new(x, y, z), border, |pos| {
                loader.block(pos).map(|block| rules.is_solid(block))
            })
        };
        player_movement(&mut self.ecs, &solid);
        let mut changes = Vec::new();
        let mut world = Bounded { loader, border };
        self.updates.tick(&mut world, &mut changes);
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::loader::tests::loader;
    use std::path::Path;
    use strata_core::chunk::Column;
    use strata_core::content::Content;
    use strata_core::coords::{ColumnPos, DEFAULT_WORLD_BORDER};
    use strata_core::pack::PackSet;

    struct World {
        game: Game,
        loader: ColumnLoader,
        content: Content,
    }

    /// Stone up to y = 63 in column (0, 0), the player standing on it at (8.5, 64, 8.5).
    fn world() -> World {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        let (content, _) = Content::load(&packs, &PackSet::default());
        let stone = block(&content, "strata:stone");
        let mut column = Column::empty(ColumnPos { x: 0, z: 0 });
        for (x, y, z) in
            (0..16).flat_map(|x| (0..64).flat_map(move |y| (0..16).map(move |z| (x, y, z))))
        {
            column.set(x, y, z, stone);
        }
        let mut loader = loader();
        loader.insert(column);
        let mut body = Body::at(DVec3::new(8.5, 64.0, 8.5));
        body.on_ground = true;
        let game = Game::new(
            BlockRules::new(&content),
            DEFAULT_WORLD_BORDER,
            body,
            GameMode::Creative,
        );
        World {
            game,
            loader,
            content,
        }
    }

    fn block(content: &Content, id: &str) -> BlockId {
        content.blocks.key(&id.parse().unwrap()).unwrap()
    }

    fn wait(world: &mut World, ticks: u32) {
        for _ in 0..ticks {
            world.game.tick(&mut world.loader);
        }
    }

    #[test]
    fn breaking_needs_reach_and_respects_the_rate_limit() {
        let mut w = world();
        let near = BlockPos::new(9, 63, 8);
        assert!(!w.game.break_block(&mut w.loader, near).is_empty());
        let next = BlockPos::new(10, 63, 8);
        assert!(
            w.game.break_block(&mut w.loader, next).is_empty(),
            "too soon"
        );
        wait(&mut w, 2);
        assert!(!w.game.break_block(&mut w.loader, next).is_empty());
        wait(&mut w, 2);
        assert!(
            w.game
                .break_block(&mut w.loader, BlockPos::new(8, 40, 8))
                .is_empty(),
            "too far"
        );
    }

    #[test]
    fn placing_never_fills_the_player_and_turns_logs() {
        let mut w = world();
        let log = block(&w.content, "strata:oak_log");
        let under_feet = BlockPos::new(8, 63, 8);
        assert!(
            w.game
                .place_block(&mut w.loader, under_feet, Face::Up, log)
                .is_empty()
        );
        let beside = BlockPos::new(11, 63, 8);
        let [(pos, placed)] = w.game.place_block(&mut w.loader, beside, Face::Up, log)[..] else {
            panic!("placed beside the player");
        };
        assert_eq!(pos, BlockPos::new(11, 64, 8));
        assert_eq!(placed, log);
        wait(&mut w, 2);
        let [(_, lying)] = w.game.place_block(&mut w.loader, pos, Face::East, log)[..] else {
            panic!("placed against the log");
        };
        assert_eq!(lying, block(&w.content, "strata:oak_log__x"));
        wait(&mut w, 2);
        let variant = block(&w.content, "strata:oak_log__x");
        assert!(
            w.game
                .place_block(&mut w.loader, beside, Face::West, variant)
                .is_empty(),
            "variants have no item"
        );
    }

    #[test]
    fn hostile_coordinates_are_refused_without_panicking() {
        let mut w = world();
        let far = BlockPos::new(i32::MAX, 10, i32::MIN);
        assert!(w.game.break_block(&mut w.loader, far).is_empty());
        assert!(
            w.game
                .place_block(&mut w.loader, far, Face::East, BlockId(1))
                .is_empty()
        );
    }

    #[test]
    fn placing_while_aiming_at_grass_replaces_the_grass() {
        let mut w = world();
        let grass = block(&w.content, "strata:tall_grass");
        let planks = block(&w.content, "strata:oak_planks");
        let plant = BlockPos::new(11, 64, 8);
        assert!(w.loader.set_block(plant, grass));
        let changes = w
            .game
            .place_block(&mut w.loader, plant, Face::North, planks);
        assert_eq!(changes, vec![(plant, planks)]);
        assert_eq!(w.loader.block(plant), Some(planks));
    }

    #[test]
    fn plants_go_with_their_ground_and_need_one() {
        let mut w = world();
        let poppy = block(&w.content, "strata:poppy");
        let (ground, plant) = (BlockPos::new(11, 63, 8), BlockPos::new(11, 64, 8));
        assert!(w.loader.set_block(plant, poppy));
        let changes = w.game.break_block(&mut w.loader, ground);
        assert_eq!(changes, vec![(ground, BlockId::AIR), (plant, BlockId::AIR)]);
        assert_eq!(w.loader.block(plant), Some(BlockId::AIR));
        wait(&mut w, 2);
        assert!(w.loader.set_block(BlockPos::new(11, 62, 8), BlockId::AIR));
        let wall = BlockPos::new(10, 63, 8);
        assert!(
            w.game
                .place_block(&mut w.loader, wall, Face::East, poppy)
                .is_empty(),
            "no ground under the cell"
        );
    }

    #[test]
    fn breaking_a_wall_lets_water_flow_and_the_changes_go_out() {
        let mut w = world();
        let water = block(&w.content, "strata:water");
        let (source, wall) = (BlockPos::new(12, 63, 8), BlockPos::new(11, 63, 8));
        assert!(w.loader.set_block(source, water));
        assert!(
            w.game.tick(&mut w.loader).is_empty(),
            "still until disturbed"
        );
        assert!(!w.game.break_block(&mut w.loader, wall).is_empty());
        let changes: Vec<BlockChange> = (0..10).flat_map(|_| w.game.tick(&mut w.loader)).collect();
        let flowing = block(&w.content, "strata:water__1");
        assert!(changes.contains(&(wall, flowing)));
        assert_eq!(w.loader.block(wall), Some(flowing));
    }
}
