//! Movement abilities given by equipment: jumps in the air, a short dash, climbing the
//! walls walked into. Part of the shared step, so the server and the client's prediction
//! agree; the body keeps what they need between ticks.

use super::body::{Body, MoveInput};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// What the worn equipment lets a body do (nothing by default).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Mobility {
    /// Jumps in the air before landing again.
    pub air_jumps: u8,
    pub air_jump_speed: f64,
    pub dash: Option<Dash>,
    /// Speed up a wall walked into, when climbing is given.
    pub climb: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dash {
    /// Blocks per second, for `ticks` ticks.
    pub speed: f64,
    pub ticks: u8,
    /// Ticks before the next one.
    pub cooldown: u16,
}

/// What a body remembers of its abilities between ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct AbilityState {
    /// Jump was held last tick: a jump in the air needs a new press.
    pub jump_held: bool,
    pub air_jumps_used: u8,
    /// Ticks of dash left, and its velocity.
    pub dash_left: u8,
    pub dash_velocity: [f64; 2],
    pub dash_cooldown: u16,
}

/// Before steering: landing gives the air jumps back, the dash cools down.
pub(super) fn begin(body: &mut Body) {
    if body.on_ground {
        body.abilities.air_jumps_used = 0;
    }
    let state = &mut body.abilities;
    state.dash_cooldown = state.dash_cooldown.saturating_sub(1);
}

/// After steering on dry land: a new press in the air jumps again, a dash press starts
/// a dash, a dash going on keeps its speed.
pub(super) fn steer(body: &mut Body, input: &MoveInput, mobility: &Mobility, wish: DVec2) {
    let pressed = input.jump && !body.abilities.jump_held;
    let state = &mut body.abilities;
    if pressed && !body.on_ground && state.air_jumps_used < mobility.air_jumps {
        state.air_jumps_used += 1;
        body.velocity.y = mobility.air_jump_speed;
    }
    if let Some(dash) = mobility.dash.filter(|_| input.dash) {
        start_dash(body, input, &dash, wish);
    }
    let state = &mut body.abilities;
    if state.dash_left > 0 {
        state.dash_left -= 1;
        body.velocity.x = state.dash_velocity[0];
        body.velocity.z = state.dash_velocity[1];
    }
}

/// Towards the movement wished, or straight ahead when standing.
fn start_dash(body: &mut Body, input: &MoveInput, dash: &Dash, wish: DVec2) {
    let state = &mut body.abilities;
    if state.dash_cooldown > 0 || state.dash_left > 0 {
        return;
    }
    let (sin, cos) = f64::from(input.yaw).sin_cos();
    let direction = wish.try_normalize().unwrap_or(DVec2::new(sin, -cos));
    let velocity = direction * dash.speed;
    state.dash_left = dash.ticks;
    state.dash_velocity = [velocity.x, velocity.y];
    state.dash_cooldown = dash.cooldown;
}

/// After moving on dry land: walking into a wall climbs it (sneaking holds on).
pub(super) fn climb(
    body: &mut Body,
    input: &MoveInput,
    mobility: &Mobility,
    wish: DVec2,
    solid: &impl Fn(i32, i32, i32) -> bool,
) {
    let Some(speed) = mobility.climb else {
        return;
    };
    if input.wish[1] <= 0.0 || !against_wall(body, wish, solid) {
        return;
    }
    body.abilities.air_jumps_used = 0;
    body.velocity.y = if input.sneak {
        0.0
    } else {
        body.velocity.y.max(speed)
    };
}

fn against_wall(body: &Body, wish: DVec2, solid: &impl Fn(i32, i32, i32) -> bool) -> bool {
    let ahead = wish.normalize_or_zero() * 0.1;
    if ahead == DVec2::ZERO {
        return false;
    }
    body.aabb()
        .translated(DVec3::new(ahead.x, 0.0, ahead.y))
        .collides(solid)
}

/// After the tick: remember whether jump was held.
pub(super) fn end(body: &mut Body, input: &MoveInput) {
    body.abilities.jump_held = input.jump;
}

#[cfg(test)]
#[path = "abilities_tests.rs"]
mod tests;
