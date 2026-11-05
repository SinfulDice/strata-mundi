//! Movement (players, boats) shared by the server (authoritative) and the client (prediction):
//! the same code on both sides, so they agree tick after tick.

mod aabb;
mod boat;
mod body;
mod swim;

pub use aabb::Aabb;
pub use boat::{
    BoatBody, BoatControls, BoatHandling, BoxSize, DEFAULT_SLIPPERINESS, liquid_surface, step_boat,
};
pub use body::{Body, EYE_HEIGHT, HEIGHT, MoveInput, MovementSettings, TICK, WIDTH, step};
pub use swim::LiquidCell;

use crate::coords::{BlockPos, WORLD_HEIGHT};

/// The world as movement sees it: what stops a body, and what it can swim in.
pub trait Terrain {
    fn solid(&self, x: i32, y: i32, z: i32) -> bool;

    fn liquid(&self, _x: i32, _y: i32, _z: i32) -> Option<LiquidCell> {
        None
    }

    /// Fraction of a sliding speed kept per tick on this block (boats; ice slides).
    fn slipperiness(&self, _x: i32, _y: i32, _z: i32) -> f64 {
        DEFAULT_SLIPPERINESS
    }
}

/// A plain solidity test is a world without liquids.
impl<F: Fn(i32, i32, i32) -> bool> Terrain for F {
    fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        self(x, y, z)
    }
}

/// A world given by two lookups: solidity and liquids.
pub struct TerrainFns<S, L> {
    pub solid: S,
    pub liquid: L,
}

impl<S, L> Terrain for TerrainFns<S, L>
where
    S: Fn(i32, i32, i32) -> bool,
    L: Fn(i32, i32, i32) -> Option<LiquidCell>,
{
    fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        (self.solid)(x, y, z)
    }

    fn liquid(&self, x: i32, y: i32, z: i32) -> Option<LiquidCell> {
        (self.liquid)(x, y, z)
    }
}

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
