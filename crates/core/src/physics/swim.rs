//! Moving inside a liquid: slower, sinking slowly, rising while jump is held.

use super::Terrain;
use super::body::{Body, GRAVITY, TICK};
use crate::content::LiquidMotion;
use glam::{DVec2, DVec3};

/// Fraction of the gap to the wished horizontal velocity closed each tick.
const SWIM_GRIP: f64 = 0.25;
/// Upward speed gained each tick while jump is held (m/s).
const SWIM_UP: f64 = 0.8;
/// Speed given when swimming against a bank that can be climbed (Minecraft's 0.3 b/t).
const CLIMB_OUT_SPEED: f64 = 6.0;
const CLIMB_OUT_PROBE: f64 = 0.6;
const CLIMB_OUT_REACH: f64 = 0.1;

/// A liquid in one cell: how it moves bodies and where its surface is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquidCell {
    pub motion: LiquidMotion,
    /// Surface height inside the cell, 0 to 1.
    pub surface: f64,
}

/// The thickest liquid the body is in, if any.
pub fn immersion(body: &Body, terrain: &impl Terrain) -> Option<LiquidMotion> {
    let aabb = body.aabb();
    let mut thickest: Option<LiquidMotion> = None;
    for x in aabb.cells(0) {
        for y in aabb.cells(1) {
            for z in aabb.cells(2) {
                let Some(cell) = terrain.liquid(x, y, z) else {
                    continue;
                };
                let touched = aabb.min.y < f64::from(y) + cell.surface;
                if touched && thickest.is_none_or(|t| cell.motion.drag < t.drag) {
                    thickest = Some(cell.motion);
                }
            }
        }
    }
    thickest
}

pub fn steer(body: &mut Body, wish: DVec2, jump: bool) {
    body.velocity.x += (wish.x - body.velocity.x) * SWIM_GRIP;
    body.velocity.z += (wish.y - body.velocity.z) * SWIM_GRIP;
    if jump {
        body.velocity.y += SWIM_UP;
    }
}

/// After moving: reduced gravity, then the liquid's drag on every axis.
pub fn sink(body: &mut Body, motion: LiquidMotion) {
    body.velocity.y -= GRAVITY * motion.gravity * TICK;
    body.velocity *= motion.drag;
}

/// Against a bank whose top is free a little higher up: a push to climb out.
pub fn climb_out(body: &mut Body, wish: DVec2, solid: &impl Fn(i32, i32, i32) -> bool) {
    let ahead = wish.normalize_or_zero() * CLIMB_OUT_REACH;
    let raised = body
        .aabb()
        .translated(DVec3::new(ahead.x, CLIMB_OUT_PROBE, ahead.y));
    if !raised.collides(solid) {
        body.velocity.y = body.velocity.y.max(CLIMB_OUT_SPEED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::{MoveInput, MovementSettings, step};

    /// Water from y = 0 to y = 9 (surface 1 block above y = 9) over a floor at y = 0, and
    /// a bank at x ≥ 5 rising to y = 9: its top is just above the water.
    struct Pool {
        motion: LiquidMotion,
    }

    impl Terrain for Pool {
        fn solid(&self, x: i32, y: i32, _z: i32) -> bool {
            y < 0 || (x >= 5 && y < 9)
        }

        fn liquid(&self, x: i32, y: i32, _z: i32) -> Option<LiquidCell> {
            ((0..9).contains(&y) && x < 5).then_some(LiquidCell {
                motion: self.motion,
                surface: 1.0,
            })
        }
    }

    const WATER: LiquidMotion = LiquidMotion {
        speed: 0.5,
        drag: 0.8,
        gravity: 0.15,
    };
    const LAVA: LiquidMotion = LiquidMotion {
        speed: 0.25,
        drag: 0.5,
        gravity: 0.25,
    };

    fn settings() -> MovementSettings {
        MovementSettings {
            can_fly: true,
            step_up: true,
            auto_jump: false,
        }
    }

    fn run(body: &mut Body, input: MoveInput, pool: &Pool, ticks: u32) {
        for _ in 0..ticks {
            step(body, &input, &settings(), pool);
        }
    }

    fn swimmer(y: f64) -> Body {
        Body::at(DVec3::new(0.5, y, 0.5))
    }

    #[test]
    fn sinks_slowly_without_input() {
        let pool = Pool { motion: WATER };
        let mut body = swimmer(6.0);
        run(&mut body, MoveInput::default(), &pool, 20);
        let speed = -body.velocity.y;
        assert!((0.3..2.0).contains(&speed), "sinking at {speed} m/s");
        assert!(body.position.y > 3.0, "{}", body.position.y);
        run(&mut body, MoveInput::default(), &pool, 200);
        assert!(body.on_ground && body.position.y.abs() < 1e-6);
    }

    #[test]
    fn holding_jump_rises_to_the_surface_and_floats() {
        let pool = Pool { motion: WATER };
        let mut body = swimmer(0.0);
        let jump = MoveInput {
            jump: true,
            ..MoveInput::default()
        };
        run(&mut body, jump, &pool, 20);
        assert!(body.velocity.y > 1.0, "rising: {}", body.velocity.y);
        run(&mut body, jump, &pool, 200);
        let y = body.position.y;
        assert!((7.0..9.5).contains(&y), "bobbing at the surface: {y}");
    }

    #[test]
    fn water_slows_and_lava_slows_more() {
        let west = MoveInput {
            wish: [0.0, 1.0],
            yaw: -std::f32::consts::FRAC_PI_2,
            ..MoveInput::default()
        };
        let distance = |motion| {
            let pool = Pool { motion };
            let mut body = swimmer(0.0);
            run(&mut body, west, &pool, 40);
            0.5 - body.position.x
        };
        let dry = {
            let mut body = swimmer(0.0);
            for _ in 0..40 {
                step(&mut body, &west, &settings(), &|_, y, _| y < 0);
            }
            0.5 - body.position.x
        };
        let (water, lava) = (distance(WATER), distance(LAVA));
        assert!(water < dry * 0.6, "water {water} vs dry {dry}");
        assert!(lava < water * 0.7, "lava {lava} vs water {water}");
        assert!(lava > 0.5, "still moving in lava: {lava}");
    }

    #[test]
    fn swimming_against_the_bank_climbs_out() {
        let pool = Pool { motion: WATER };
        let mut body = swimmer(8.2);
        let east = MoveInput {
            wish: [0.0, 1.0],
            yaw: std::f32::consts::FRAC_PI_2,
            jump: true,
            ..MoveInput::default()
        };
        run(&mut body, east, &pool, 60);
        assert!(
            body.position.x > 5.0 && body.position.y >= 9.0 - 1e-6,
            "{:?}",
            body.position
        );
    }

    #[test]
    fn flying_ignores_the_water() {
        let pool = Pool { motion: WATER };
        let mut body = swimmer(4.0);
        body.flying = true;
        run(&mut body, MoveInput::default(), &pool, 20);
        assert!((body.position.y - 4.0).abs() < 1e-3 && body.flying);
    }
}
