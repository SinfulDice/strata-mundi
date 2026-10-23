//! Systems: functions run once per tick over the ECS storages.

use crate::ecs::Ecs;
use strata_core::physics::{MoveInput, step};

/// Inputs a client may bank to catch up after a stall (one more is earned each tick).
const MAX_INPUT_BUDGET: u8 = 3;
/// Without input for this many ticks (half a second), the body moves on its own, so a
/// silent client cannot hover.
const IDLE_TICKS: u32 = 10;

/// Applies each player's queued inputs with the shared physics, at the pace of the clock,
/// and records the last input sequence applied.
pub fn player_movement(ecs: &mut Ecs, solid: &impl Fn(i32, i32, i32) -> bool) {
    for (entity, control) in ecs.controls.iter_mut() {
        let Some(body) = ecs.bodies.get_mut(entity) else {
            continue;
        };
        control.input_budget = (control.input_budget + 1).min(MAX_INPUT_BUDGET);
        let mut applied = false;
        while control.input_budget > 0 {
            let Some((sequence, input)) = control.pending.pop_front() else {
                break;
            };
            step(body, &input, &control.movement, solid);
            control.last_sequence = sequence;
            control.input_budget -= 1;
            applied = true;
        }
        control.idle_ticks = if applied { 0 } else { control.idle_ticks + 1 };
        if control.idle_ticks > IDLE_TICKS {
            step(body, &MoveInput::default(), &control.movement, solid);
        }
    }
}
