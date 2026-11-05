//! Systems: functions run once per tick over the ECS storages.

use crate::ecs::{Ecs, Entity};
use crate::rules::BlockRules;
use glam::DVec3;
use strata_core::physics::{BoatControls, MoveInput, Terrain, step, step_boat};
use strata_core::updates::BlockChange;

/// Inputs a client may bank to catch up after a stall (one more is earned each tick).
const MAX_INPUT_BUDGET: u8 = 3;
/// Without input for this many ticks (half a second), the body moves on its own, so a
/// silent client cannot hover.
const IDLE_TICKS: u32 = 10;
/// Boats closer than this to their last sent position and yaw are not sent again.
const MOVE_EPSILON: f64 = 1e-4;
/// An empty boat this many ticks still (one second) stops being simulated.
pub const SLEEP_TICKS: u32 = 20;

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

/// Moves every awake boat one tick, then sits the riders in them. Returns the boats that
/// moved enough to be sent again.
pub fn boat_movement(ecs: &mut Ecs, rules: &BlockRules, terrain: &impl Terrain) -> Vec<Entity> {
    let mut moved = Vec::new();
    for (entity, boat) in ecs.boats.iter_mut() {
        let Some(kind) = rules.entity(boat.kind) else {
            continue;
        };
        let Some(handling) = kind.boat else {
            continue;
        };
        if boat.pilot.is_none() && boat.still_ticks >= SLEEP_TICKS {
            continue;
        }
        let before = boat.body.position;
        step_boat(&mut boat.body, boat.controls, &handling, kind.size, terrain);
        let still = boat.body.position.distance(before) < MOVE_EPSILON
            && boat.body.velocity.length() < MOVE_EPSILON;
        boat.still_ticks = match (still, boat.pilot) {
            (true, None) => boat.still_ticks + 1,
            _ => 0,
        };
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

/// Wakes the sleeping boats next to a changed block: the water may have left, or the
/// ground under them.
pub fn wake_boats(ecs: &mut Ecs, rules: &BlockRules, changed: &[BlockChange]) {
    if changed.is_empty() {
        return;
    }
    for (_, boat) in ecs.boats.iter_mut() {
        if boat.still_ticks < SLEEP_TICKS {
            continue;
        }
        let Some(kind) = rules.entity(boat.kind) else {
            continue;
        };
        let reach = boat.body.aabb(kind.size);
        let (min, max) = (reach.min - DVec3::ONE, reach.max + DVec3::ONE);
        let near = changed.iter().any(|(pos, _)| {
            let cell = DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z));
            (cell + DVec3::ONE).cmpge(min).all() && cell.cmple(max).all()
        });
        if near {
            boat.still_ticks = 0;
        }
    }
}
