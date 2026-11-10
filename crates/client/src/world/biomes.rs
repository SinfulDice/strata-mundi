//! Biomes of the loaded columns and their colours, blended for meshing.

use super::{ClientWorld, around, column_of, local};
use crate::mesher::ColumnTints;
use std::collections::HashMap;
use std::sync::Arc;
use strata_core::chunk::BiomeId;
use strata_core::content::BiomeTints;
use strata_core::coords::{ColumnPos, SECTION_SIZE};

/// Colours of the columns meshed lately, for one blend radius (biomes never change).
#[derive(Debug, Default)]
pub(super) struct TintCache {
    radius: u32,
    columns: HashMap<ColumnPos, Arc<ColumnTints>>,
}

impl TintCache {
    pub(super) fn forget(&mut self, pos: ColumnPos) {
        self.columns.remove(&pos);
    }
}

impl ClientWorld {
    /// Colours of the biomes, by `BiomeId`; the caller remeshes.
    pub fn set_biome_tints(&mut self, tints: Arc<[BiomeTints]>) {
        self.biome_tints = tints;
        self.tints.get_mut().columns.clear();
    }

    pub fn biome_tints(&self) -> &[BiomeTints] {
        &self.biome_tints
    }

    pub(super) fn forget_tints_around(&mut self, pos: ColumnPos) {
        let tints = &mut self.tints.get_mut().columns;
        for dz in -1..=1 {
            for dx in -1..=1 {
                tints.remove(&ColumnPos {
                    x: pos.x + dx,
                    z: pos.z + dz,
                });
            }
        }
    }

    /// Colours of a column blended with its neighbours over `radius` blocks, computed
    /// once per radius.
    pub(super) fn column_tints(&self, cx: i32, cz: i32, radius: u32) -> Option<Arc<ColumnTints>> {
        if self.biome_tints.is_empty() {
            return None;
        }
        let pos = ColumnPos { x: cx, z: cz };
        {
            let mut cache = self.tints.borrow_mut();
            if cache.radius != radius {
                cache.radius = radius;
                cache.columns.clear();
            }
            if let Some(tints) = cache.columns.get(&pos) {
                return Some(Arc::clone(tints));
            }
        }
        let maps = around(&self.biomes, cx, cz);
        let centre = maps[1][1]?;
        let tints = Arc::new(ColumnTints::new(radius, &self.biome_tints, |x, z| {
            let (dx, lx) = split_wide(x);
            let (dz, lz) = split_wide(z);
            maps[dz][dx].unwrap_or(centre).get(lx, lz)
        }));
        let columns = &mut self.tints.borrow_mut().columns;
        columns.insert(pos, Arc::clone(&tints));
        Some(tints)
    }

    /// `None` where nothing is loaded.
    pub fn biome_at(&self, x: i32, z: i32) -> Option<BiomeId> {
        let map = self.biomes.get(&column_of(x, z))?;
        Some(map.get(local(x), local(z)))
    }
}

/// `-16..32` relative to a column → (neighbour index 0..3, local coordinate).
fn split_wide(c: i32) -> (usize, u32) {
    (
        (c.div_euclid(SECTION_SIZE) + 1).clamp(0, 2) as usize,
        local(c),
    )
}
