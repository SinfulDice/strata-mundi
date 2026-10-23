use super::Aabb;
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// Fixed simulation step: 20 ticks per second.
pub const TICK: f64 = 0.05;
pub const WIDTH: f64 = 0.6;
pub const HEIGHT: f64 = 1.8;
pub const EYE_HEIGHT: f64 = 1.62;
const GRAVITY: f64 = 32.0;
const TERMINAL_SPEED: f64 = 78.4;
const JUMP_SPEED: f64 = 8.4;
const WALK_SPEED: f64 = 4.317;
const SPRINT_SPEED: f64 = 5.612;
const SNEAK_SPEED: f64 = 1.31;
const FLY_SPEED: f64 = 10.9;
const FLY_VERTICAL_SPEED: f64 = 7.5;
/// Fraction of the gap to the wished velocity closed each tick, on ground and in the air.
const GROUND_GRIP: f64 = 0.6;
const AIR_GRIP: f64 = 0.1;
const FLY_GRIP: f64 = 0.3;
/// Longest move done in one go: thinner than any block, so nothing is ever skipped.
const MAX_SUBSTEP: f64 = 0.4;
const STEP_HEIGHT: f64 = 0.5;

/// What the player wants this tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveInput {
    /// x = right, y = forward, length at most 1.
    pub wish: [f32; 2],
    /// Radians, 0 = north (-Z).
    pub yaw: f32,
    /// Radians, positive = up. Not used for moving; kept with the player (saves, aiming).
    pub pitch: f32,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    /// Creative: start or stop flying (double jump).
    pub toggle_flight: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MovementSettings {
    pub can_fly: bool,
    /// Climb half-block steps without jumping.
    pub step_up: bool,
    /// Jump automatically onto a one-block step when walking into it.
    pub auto_jump: bool,
}

/// Position (feet, bottom centre) and motion of a moving body.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Body {
    pub position: DVec3,
    pub velocity: DVec3,
    pub on_ground: bool,
    pub flying: bool,
}

impl Body {
    pub fn at(position: DVec3) -> Self {
        Self {
            position,
            velocity: DVec3::ZERO,
            on_ground: false,
            flying: false,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::standing(self.position, WIDTH, HEIGHT)
    }
}

/// Advances `body` by one tick.
pub fn step(
    body: &mut Body,
    input: &MoveInput,
    settings: &MovementSettings,
    solid: &impl Fn(i32, i32, i32) -> bool,
) {
    if input.toggle_flight && settings.can_fly {
        body.flying = !body.flying;
    }
    if !settings.can_fly {
        body.flying = false;
    }
    let wish = wished_velocity(body, input);
    steer(body, wish, input);
    let delta = body.velocity * TICK;
    let blocked = move_body(body, delta, input, settings, solid);
    fall(body);
    let may_jump = settings.auto_jump && body.on_ground && !body.flying && !input.sneak;
    if blocked && may_jump && faces_one_block_step(body, wish, solid) {
        body.velocity.y = JUMP_SPEED;
    }
    if body.flying && body.on_ground {
        body.flying = false;
    }
}

impl Body {
    /// Where the eyes are, for the camera and for aiming.
    pub fn eyes(&self) -> DVec3 {
        self.position + DVec3::Y * EYE_HEIGHT
    }
}

fn wished_velocity(body: &Body, input: &MoveInput) -> DVec2 {
    let speed = match (body.flying, input.sprint, input.sneak) {
        (true, true, _) => FLY_SPEED * 2.0,
        (true, false, _) => FLY_SPEED,
        (false, _, true) => SNEAK_SPEED,
        (false, true, false) => SPRINT_SPEED,
        (false, false, false) => WALK_SPEED,
    };
    let (sin, cos) = f64::from(input.yaw).sin_cos();
    let forward = DVec2::new(sin, -cos);
    let right = DVec2::new(cos, sin);
    let wish = DVec2::new(f64::from(input.wish[0]), f64::from(input.wish[1])).clamp_length_max(1.0);
    (right * wish.x + forward * wish.y) * speed
}

fn steer(body: &mut Body, wish: DVec2, input: &MoveInput) {
    let grip = match (body.flying, body.on_ground) {
        (true, _) => FLY_GRIP,
        (false, true) => GROUND_GRIP,
        (false, false) => AIR_GRIP,
    };
    body.velocity.x += (wish.x - body.velocity.x) * grip;
    body.velocity.z += (wish.y - body.velocity.z) * grip;
    if body.flying {
        let vertical = f64::from(u8::from(input.jump)) - f64::from(u8::from(input.sneak));
        body.velocity.y += (vertical * FLY_VERTICAL_SPEED - body.velocity.y) * FLY_GRIP * 2.0;
        return;
    }
    if input.jump && body.on_ground {
        body.velocity.y = JUMP_SPEED;
    }
}

/// Applied after moving, so a jump starts at full speed (Minecraft's order).
fn fall(body: &mut Body) {
    if !body.flying {
        body.velocity.y = (body.velocity.y - GRAVITY * TICK).max(-TERMINAL_SPEED);
    }
}

/// Moves in sub-steps, axis by axis; returns whether a wall stopped horizontal motion.
fn move_body(
    body: &mut Body,
    delta: DVec3,
    input: &MoveInput,
    settings: &MovementSettings,
    solid: &impl Fn(i32, i32, i32) -> bool,
) -> bool {
    let steps = (delta.length() / MAX_SUBSTEP).ceil().max(1.0);
    let part = delta / steps;
    let mut blocked = false;
    body.on_ground = false;
    for _ in 0..steps as u32 {
        let fell = move_axis(body, 1, part.y, solid);
        if fell != part.y {
            body.on_ground |= part.y < 0.0;
            body.velocity.y = 0.0;
        }
        let edge_guard = input.sneak && !body.flying && body.on_ground;
        for axis in [0, 2] {
            if edge_guard && !keeps_footing(body, axis, part[axis], solid) {
                body.velocity[axis] = 0.0;
                continue;
            }
            let wanted = part[axis];
            let moved = move_axis(body, axis, wanted, solid);
            if moved != wanted
                && !(settings.step_up && try_step_up(body, axis, wanted - moved, solid))
            {
                body.velocity[axis] = 0.0;
                blocked = true;
            }
        }
    }
    blocked
}

fn move_axis(
    body: &mut Body,
    axis: usize,
    distance: f64,
    solid: &impl Fn(i32, i32, i32) -> bool,
) -> f64 {
    let moved = body.aabb().sweep(axis, distance, solid);
    body.position[axis] += moved;
    moved
}

/// Sneaking never walks off an edge: refuse a move that leaves nothing underfoot.
fn keeps_footing(
    body: &Body,
    axis: usize,
    distance: f64,
    solid: &impl Fn(i32, i32, i32) -> bool,
) -> bool {
    let mut offset = DVec3::ZERO;
    offset[axis] = distance;
    body.aabb().translated(offset).supported(0.6, solid)
}

/// Auto-jump only climbs single blocks: something in the way at foot level, free space
/// one block higher (never against a wall).
fn faces_one_block_step(body: &Body, wish: DVec2, solid: &impl Fn(i32, i32, i32) -> bool) -> bool {
    let ahead = wish.normalize_or_zero() * 0.3;
    let probe = body.aabb().translated(DVec3::new(ahead.x, 0.0, ahead.y));
    probe.collides(solid) && !probe.translated(DVec3::Y * 1.0).collides(solid)
}

/// Half-block steps: lift, retry the blocked part, settle back down.
fn try_step_up(
    body: &mut Body,
    axis: usize,
    remaining: f64,
    solid: &impl Fn(i32, i32, i32) -> bool,
) -> bool {
    if !body.on_ground || body.flying {
        return false;
    }
    let saved = body.position;
    let lifted = move_axis(body, 1, STEP_HEIGHT, solid);
    let moved = move_axis(body, axis, remaining, solid);
    if moved.abs() < remaining.abs() * 0.5 {
        body.position = saved;
        return false;
    }
    move_axis(body, 1, -lifted, solid);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> MovementSettings {
        MovementSettings {
            can_fly: true,
            step_up: true,
            auto_jump: false,
        }
    }

    /// Flat floor at y = 0 and a wall at x = 5.
    fn world(x: i32, y: i32, _z: i32) -> bool {
        y < 0 || x == 5
    }

    fn run(body: &mut Body, input: MoveInput, ticks: u32) {
        for _ in 0..ticks {
            step(body, &input, &settings(), &world);
        }
    }

    #[test]
    fn falls_and_lands_on_the_floor() {
        let mut body = Body::at(DVec3::new(0.5, 10.0, 0.5));
        run(&mut body, MoveInput::default(), 60);
        assert!(body.on_ground);
        assert!(body.position.y.abs() < 1e-6, "{}", body.position.y);
    }

    #[test]
    fn never_goes_through_a_wall_even_very_fast() {
        let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
        body.velocity.x = 500.0;
        body.flying = true;
        step(&mut body, &MoveInput::default(), &settings(), &world);
        assert!(
            body.position.x <= 5.0 - WIDTH / 2.0 + 1e-6,
            "{}",
            body.position.x
        );
        assert_eq!(body.velocity.x, 0.0);
    }

    #[test]
    fn walking_east_stops_at_the_wall() {
        let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
        let east = MoveInput {
            wish: [0.0, 1.0],
            yaw: std::f32::consts::FRAC_PI_2,
            ..MoveInput::default()
        };
        run(&mut body, east, 100);
        assert!((body.position.x - (5.0 - WIDTH / 2.0)).abs() < 1e-6);
    }

    #[test]
    fn jumps_about_a_block_and_a_quarter() {
        let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
        run(&mut body, MoveInput::default(), 2);
        let jump = MoveInput {
            jump: true,
            ..MoveInput::default()
        };
        step(&mut body, &jump, &settings(), &world);
        let mut highest: f64 = 0.0;
        for _ in 0..20 {
            step(&mut body, &MoveInput::default(), &settings(), &world);
            highest = highest.max(body.position.y);
        }
        assert!((1.1..1.4).contains(&highest), "{highest}");
    }

    #[test]
    fn sneaking_does_not_fall_off_an_edge() {
        let ledge = |x: i32, y: i32, _z: i32| y < 0 && x < 1;
        let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
        let input = MoveInput {
            wish: [0.0, 1.0],
            yaw: std::f32::consts::FRAC_PI_2,
            sneak: true,
            ..MoveInput::default()
        };
        for _ in 0..100 {
            step(&mut body, &input, &settings(), &ledge);
        }
        assert!(body.on_ground && body.position.y.abs() < 1e-6);
        assert!(body.position.x < 1.0 + WIDTH / 2.0);
    }

    #[test]
    fn auto_jump_climbs_one_block() {
        let step_world = |x: i32, y: i32, _z: i32| y < 0 || (x >= 3 && y < 1);
        let mut auto = settings();
        auto.auto_jump = true;
        let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
        let east = MoveInput {
            wish: [0.0, 1.0],
            yaw: std::f32::consts::FRAC_PI_2,
            ..MoveInput::default()
        };
        for _ in 0..60 {
            step(&mut body, &east, &auto, &step_world);
        }
        assert!(
            body.position.x > 4.0 && (body.position.y - 1.0).abs() < 1e-6,
            "{:?}",
            body.position
        );
    }

    #[test]
    fn auto_jump_ignores_walls_and_sneaking() {
        let wall = |x: i32, y: i32, _z: i32| y < 0 || (x >= 3 && y < 2);
        let mut auto = settings();
        auto.auto_jump = true;
        let east = MoveInput {
            wish: [0.0, 1.0],
            yaw: std::f32::consts::FRAC_PI_2,
            ..MoveInput::default()
        };
        let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
        for _ in 0..40 {
            step(&mut body, &east, &auto, &wall);
            assert!(body.position.y < 0.1, "jumped against a wall");
        }
        let step_world = |x: i32, y: i32, _z: i32| y < 0 || (x >= 3 && y < 1);
        let sneaking = MoveInput {
            sneak: true,
            ..east
        };
        let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
        for _ in 0..80 {
            step(&mut body, &sneaking, &auto, &step_world);
        }
        assert!(body.position.y < 0.1);
    }

    #[test]
    fn landing_ends_flight() {
        let mut body = Body::at(DVec3::new(0.5, 3.0, 0.5));
        body.flying = true;
        run(
            &mut body,
            MoveInput {
                sneak: true,
                ..MoveInput::default()
            },
            40,
        );
        assert!(!body.flying && body.on_ground);
    }
}
