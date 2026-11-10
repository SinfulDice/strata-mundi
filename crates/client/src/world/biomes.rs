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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesher::{BlockTable, MeshOptions, PackedVertex, mesh_section};
    use strata_core::chunk::{BiomeMap, Column};
    use strata_core::content::{Block, BlockId, BlockTint, RenderKind, Tint, TintFaces, TintKind};

    const GREEN: Tint = Tint([90, 150, 90]);
    const YELLOW: Tint = Tint([190, 140, 60]);

    /// Ids: 0 air, 1 grass (tinted top).
    fn table() -> BlockTable {
        let grass = Block {
            render: RenderKind::Opaque,
            tint: Some(BlockTint {
                kind: TintKind::Grass,
                faces: TintFaces::TopFringe,
            }),
            ..Block::air()
        };
        BlockTable::new([Block::air(), grass].iter())
    }

    /// A flat grass floor at y = 3 over columns x -1..=2, z -1..=1; biome 1 from x = 20.
    fn world() -> ClientWorld {
        let mut world = ClientWorld::default();
        world.set_biome_tints(
            [GREEN, YELLOW]
                .map(|grass| BiomeTints {
                    kinds: [grass, Tint::NEUTRAL, Tint::NEUTRAL],
                    fog: Tint::NEUTRAL,
                })
                .to_vec()
                .into(),
        );
        for cx in -1..=2 {
            for cz in -1..=1 {
                let pos = ColumnPos { x: cx, z: cz };
                let mut column = Column::empty(pos);
                for x in 0..16 {
                    for z in 0..16 {
                        column.set(x, 3, z, BlockId(1));
                    }
                }
                let biomes = BiomeMap::from_fn(|x, _| BiomeId(u16::from(cx * 16 + x as i32 >= 20)));
                world.insert(column, biomes);
            }
        }
        world
    }

    /// Colour of each top vertex of a section on its z = 0 edge, by world x.
    fn top_colours(world: &ClientWorld, cx: i32, blend: u32) -> Vec<(i32, Tint)> {
        let table = table();
        let options = MeshOptions {
            biome_blend: blend,
            ..MeshOptions::default()
        };
        let mesh = mesh_section(
            &world.neighborhood((cx, 0, 0), &table, blend),
            &table,
            options,
        );
        let mut colours: Vec<(i32, Tint)> = mesh
            .opaque
            .vertices
            .iter()
            .filter(|v| v.position()[1] == 4 && v.position()[2] == 0)
            .map(|v: &PackedVertex| (cx * 16 + v.position()[0] as i32, v.tint().0))
            .collect();
        colours.sort_by_key(|(x, _)| *x);
        colours.dedup();
        colours
    }

    #[test]
    fn grass_colour_is_continuous_across_biome_and_column_borders() {
        let world = world();
        let mut row = top_colours(&world, 0, 4);
        row.extend(top_colours(&world, 1, 4));
        let at = |x: i32| row.iter().filter(move |(vx, _)| *vx == x).map(|(_, t)| *t);
        let shared: Vec<Tint> = at(16).collect();
        assert!(!shared.is_empty());
        assert!(
            shared.iter().all(|t| *t == shared[0]),
            "both columns agree at x = 16"
        );
        assert_eq!(at(0).next(), Some(GREEN));
        assert_eq!(at(32).next(), Some(YELLOW));
        for pair in row.windows(2) {
            let step = (0..3)
                .map(|c| pair[0].1.0[c].abs_diff(pair[1].1.0[c]))
                .max();
            assert!(step.unwrap_or(0) <= 15, "{pair:?}");
        }
    }

    #[test]
    fn without_blending_each_block_keeps_its_biome_colour() {
        let world = world();
        let row = top_colours(&world, 1, 0);
        let colours = |x: i32| row.iter().filter(move |(vx, _)| *vx == x).map(|(_, t)| *t);
        assert!(colours(19).all(|t| t == GREEN));
        assert!(colours(21).all(|t| t == YELLOW));
        assert_eq!(
            colours(20).count(),
            2,
            "a sharp edge: both colours at x = 20"
        );
    }

    #[test]
    fn uniform_colours_still_merge() {
        let world = world();
        let table = table();
        let mesh = mesh_section(
            &world.neighborhood((-1, 0, 0), &table, 4),
            &table,
            MeshOptions::default(),
        );
        let tops = mesh
            .opaque
            .vertices
            .chunks(4)
            .filter(|q| q.iter().all(|v| v.position()[1] == 4));
        assert_eq!(
            tops.count(),
            1,
            "one green biome far from the border: one merged top"
        );
        let tint = |v: &PackedVertex| v.tint();
        let face = |v: &&PackedVertex| (v.geometry >> 15) & 7;
        let bottom = mesh.opaque.vertices.iter().filter(|v| face(v) == 1);
        assert!(
            bottom.map(tint).all(|t| t == (Tint::NEUTRAL, false)),
            "dirt below"
        );
        let west = mesh.opaque.vertices.iter().filter(|v| face(v) == 5);
        assert!(
            west.map(tint).all(|t| t == (GREEN, true)),
            "fringe on the sides"
        );
    }
}
