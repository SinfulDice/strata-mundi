//! Home-made ECS: entities are generational handles, each component type lives in its
//! own dense `Storage`, and systems are plain functions that walk those storages.
//! Adding a component = adding a storage field to `Ecs`.

mod entity;
mod storage;

pub use entity::{Entities, Entity};
pub use storage::Storage;

use std::collections::VecDeque;
use strata_core::content::EntityTypeId;
use strata_core::physics::{BoatBody, BoatControls, Body, MoveInput, MovementSettings};
use strata_protocol::{GameMode, HotbarState};

/// Every component storage of the simulation.
#[derive(Debug, Default)]
pub struct Ecs {
    pub entities: Entities,
    pub bodies: Storage<Body>,
    pub controls: Storage<PlayerControl>,
    pub hotbars: Storage<HotbarState>,
    pub boats: Storage<Boat>,
    /// What each riding player sits in.
    pub mounts: Storage<Entity>,
}

/// A boat entity: its type, motion, the pilot's last wishes and who sits in it.
#[derive(Debug, Clone)]
pub struct Boat {
    pub kind: EntityTypeId,
    pub body: BoatBody,
    pub controls: BoatControls,
    pub pilot: Option<Entity>,
    /// Position and yaw last sent to clients, to send only changes.
    pub sent: (glam::DVec3, f32),
}

impl Boat {
    pub fn new(kind: EntityTypeId, body: BoatBody) -> Self {
        Self {
            kind,
            body,
            controls: BoatControls::default(),
            pilot: None,
            sent: (body.position, body.yaw),
        }
    }
}

/// Inputs received from a player's client, applied one per physics step.
#[derive(Debug, Clone)]
pub struct PlayerControl {
    pub pending: VecDeque<(u32, MoveInput)>,
    pub last_sequence: u32,
    pub movement: MovementSettings,
    pub mode: GameMode,
    /// Inputs that may still be applied: one more per tick, capped, so a client can catch
    /// up after a stall but never move faster than the clock on average.
    pub input_budget: u8,
    /// Ticks without any input; past a limit the body moves on its own (it falls).
    pub idle_ticks: u32,
    /// Where the player looks (yaw, pitch), from the last input.
    pub look: [f32; 2],
    /// Tick of the last block broken or placed (rate limit).
    pub last_action: Option<u64>,
    /// Sneak was held in the last input: getting off a boat needs a new press.
    pub sneaking: bool,
}

impl PlayerControl {
    pub fn new(movement: MovementSettings, mode: GameMode) -> Self {
        Self {
            pending: VecDeque::new(),
            last_sequence: 0,
            movement,
            mode,
            input_budget: 0,
            idle_ticks: 0,
            look: [0.0; 2],
            last_action: None,
            sneaking: false,
        }
    }
}

impl Ecs {
    pub fn despawn(&mut self, entity: Entity) {
        self.bodies.remove(entity);
        self.controls.remove(entity);
        self.hotbars.remove(entity);
        self.boats.remove(entity);
        self.mounts.remove(entity);
        self.entities.despawn(entity);
    }
}
