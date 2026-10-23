//! Player movement shared by the server (authoritative) and the client (prediction):
//! the same code on both sides, so they agree tick after tick.

mod aabb;
mod body;

pub use aabb::Aabb;
pub use body::{Body, EYE_HEIGHT, HEIGHT, MoveInput, MovementSettings, TICK, WIDTH, step};

use crate::coords::{BlockPos, WORLD_HEIGHT};

/// What counts as solid for movement, the same on both sides: below the world and beyond
/// the border is solid, above it is air. `block` tells whether a loaded block stops
/// movement, `None` when its column is not loaded (solid, so nobody falls through a world
/// that has not arrived yet).
pub fn solid_at(pos: BlockPos, border: i32, block: impl Fn(BlockPos) -> Option<bool>) -> bool {
    if pos.y < 0 || !pos.is_inside_border(border) {
        return true;
    }
    pos.y < WORLD_HEIGHT && block(pos).unwrap_or(true)
}
