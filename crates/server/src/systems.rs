//! Systems: functions run once per tick over the ECS storages.

use crate::ecs::{Ecs, Entity};
use crate::rules::BlockRules;
use glam::DVec3;
use strata_core::physics::{BoatControls, MoveInput, Terrain, step, step_boat};

/// Inputs a client may bank to catch up after a stall (one more is earned each tick).
const MAX_INPUT_BUDGET: u8 = 3;
/// Without input for this many ticks (half a second), the body moves on its own, so a
/// silent client cannot hover.
const IDLE_TICKS: u32 = 10;
/// Boats closer than this to their last sent position and yaw are not sent again.
const MOVE_EPSILON: f64 = 1e-4;

/// Applies each player's queued inputs with the shared physics, at the pace of the clock,
/// and records the last input sequence applied. A riding player's inputs steer the boat
/// instead; a new sneak press asks to get off, returned for the caller to settle.
pub fn player_movement(ecs: &mut Ecs, terrain: &impl Terrain) -> Vec<Entity> {
    let mut dismounts = Vec::new();
    for (entity, control) in ecs.controls.iter_mut() {
        let Some(body) = ecs.bodies.get_mut(entity) else {
            continue;
        };
        let mut boat = ecs
            .mounts
            .get(entity)
            .and_then(|&mount| ecs.boats.get_mut(mount));
        control.input_budget = (control.input_budget + 1).min(MAX_INPUT_BUDGET);
        let mut applied = false;
        while control.input_budget > 0 {
            let Some((sequence, input)) = control.pending.pop_front() else {
                break;
            };
            let pressed_sneak = input.sneak && !control.sneaking;
            control.sneaking = input.sneak;
            match boat.as_deref_mut() {
                Some(boat) => boat.controls = steering(&input),
                None => step(body, &input, &control.movement, terrain),
            }
            control.last_sequence = sequence;
            control.look = [input.yaw, input.pitch];
            control.input_budget -= 1;
            applied = true;
            if boat.is_some() && pressed_sneak {
                dismounts.push(entity);
                break;
            }
        }
        control.idle_ticks = if applied { 0 } else { control.idle_ticks + 1 };
        if control.idle_ticks > IDLE_TICKS {
            match boat {
                Some(boat) => boat.controls = BoatControls::default(),
                None => step(body, &MoveInput::default(), &control.movement, terrain),
            }
        }
    }
    dismounts
}

/// Forward/backward paddles, left/right turns.
fn steering(input: &MoveInput) -> BoatControls {
    BoatControls {
        forward: input.wish[1],
        turn: input.wish[0],
    }
}

/// Moves every boat one tick, then sits the riders in them. Returns the boats that moved
/// enough to be sent again.
pub fn boat_movement(ecs: &mut Ecs, rules: &BlockRules, terrain: &impl Terrain) -> Vec<Entity> {
    let mut moved = Vec::new();
    for (entity, boat) in ecs.boats.iter_mut() {
        let Some(kind) = rules.entity(boat.kind) else {
            continue;
        };
        let Some(handling) = kind.boat else {
            continue;
        };
        step_boat(&mut boat.body, boat.controls, &handling, kind.size, terrain);
        let (position, yaw) = boat.sent;
        let turned = f64::from((boat.body.yaw - yaw).abs());
        if boat.body.position.distance(position) > MOVE_EPSILON || turned > MOVE_EPSILON {
            boat.sent = (boat.body.position, boat.body.yaw);
            moved.push(entity);
        }
        if let Some(rider) = boat.pilot.and_then(|pilot| ecs.bodies.get_mut(pilot)) {
            rider.position = boat.body.position + DVec3::Y * handling.seat;
            rider.velocity = boat.body.velocity;
            rider.on_ground = false;
            rider.flying = false;
        }
    }
    moved
}
