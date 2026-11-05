//! The loaded world as movement sees it, with the same rules as the client.

use crate::loader::ColumnLoader;
use crate::rules::BlockRules;
use strata_core::coords::BlockPos;
use strata_core::physics::{DEFAULT_SLIPPERINESS, LiquidCell, Terrain, solid_at};

pub struct WorldTerrain<'a> {
    pub loader: &'a ColumnLoader,
    pub rules: &'a BlockRules,
    pub border: i32,
}

impl Terrain for WorldTerrain<'_> {
    fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        solid_at(BlockPos::new(x, y, z), self.border, |pos| {
            self.loader
                .block(pos)
                .map(|block| self.rules.is_solid(block))
        })
    }

    fn liquid(&self, x: i32, y: i32, z: i32) -> Option<LiquidCell> {
        let block = self.loader.block(BlockPos::new(x, y, z))?;
        self.rules.liquid(block)
    }

    fn slipperiness(&self, x: i32, y: i32, z: i32) -> f64 {
        self.loader
            .block(BlockPos::new(x, y, z))
            .map_or(DEFAULT_SLIPPERINESS, |block| self.rules.slipperiness(block))
    }
}
