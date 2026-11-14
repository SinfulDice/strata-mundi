//! Items lying on the ground: they fall, slide to a stop and float up in liquids.

use super::{Aabb, Terrain};
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// Blocks per tick², per tick.
const GRAVITY: f64 = 0.04;
const AIR_DRAG: f64 = 0.98;
const GROUND_FRICTION: f64 = 0.6;
const BUOYANCY: f64 = 0.05;
const LIQUID_DRAG: f64 = 0.8;
/// Below this speed on the ground, the item rests (and is not simulated until woken).
const REST_SPEED: f64 = 1e-3;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ItemBody {
    /// Bottom centre.
    pub position: DVec3,
    pub velocity: DVec3,
    pub on_ground: bool,
}

impl ItemBody {
    pub fn at(position: DVec3, velocity: DVec3) -> Self {
        Self {
            position,
            velocity,
            on_ground: false,
        }
    }

    pub fn aabb(&self, size: f64) -> Aabb {
        Aabb::standing(self.position, size, size)
    }

    /// Whether it lies still on the ground.
    pub fn is_resting(&self) -> bool {
        self.on_ground && self.velocity.length_squared() < REST_SPEED * REST_SPEED
    }
}

/// One tick of motion; returns whether the item moved.
pub fn step_item(body: &mut ItemBody, size: f64, terrain: &impl Terrain) -> bool {
    let solid = |x, y, z| terrain.solid(x, y, z);
    let cell = body.position + DVec3::Y * (size / 2.0);
    let (x, y, z) = (
        cell.x.floor() as i32,
        cell.y.floor() as i32,
        cell.z.floor() as i32,
    );
    if terrain.liquid(x, y, z).is_some() {
        body.velocity = body.velocity * LIQUID_DRAG + DVec3::Y * BUOYANCY;
    } else {
        body.velocity.y -= GRAVITY;
        body.velocity *= AIR_DRAG;
    }
    if body.aabb(size).collides(&solid) {
        body.position.y = body.position.y.floor() + 1.0;
        body.velocity = DVec3::ZERO;
    }
    let start = body.position;
    let falling = body.velocity.y;
    for axis in [1, 0, 2] {
        let wanted = body.velocity[axis];
        let moved = body.aabb(size).sweep(axis, wanted, &solid);
        body.position[axis] += moved;
        if moved != wanted {
            body.velocity[axis] = 0.0;
        }
    }
    body.on_ground = falling < 0.0 && body.velocity.y == 0.0;
    if body.on_ground {
        body.velocity.x *= GROUND_FRICTION;
        body.velocity.z *= GROUND_FRICTION;
    }
    if body.is_resting() {
        body.velocity = DVec3::ZERO;
    }
    body.position != start
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_falls_and_comes_to_rest_on_the_ground() {
        let floor = |_: i32, y: i32, _: i32| y < 64;
        let mut body = ItemBody::at(DVec3::new(0.5, 70.0, 0.5), DVec3::new(0.2, 0.2, 0.0));
        for _ in 0..200 {
            step_item(&mut body, 0.25, &floor);
        }
        assert!((body.position.y - 64.0).abs() < 1e-9);
        assert!(body.is_resting());
        assert!(
            !step_item(&mut body, 0.25, &floor),
            "a resting item stays put"
        );
    }

    #[test]
    fn an_item_inside_a_block_pops_out_on_top() {
        let floor = |_: i32, y: i32, _: i32| y < 64;
        let mut body = ItemBody::at(DVec3::new(0.5, 63.2, 0.5), DVec3::ZERO);
        step_item(&mut body, 0.25, &floor);
        assert!(body.position.y >= 64.0);
    }
}
