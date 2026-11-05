//! The client's copy of the blocks around the player, as sent by the server.

use crate::entities::ClientEntities;
use crate::mesher::{BlockTable, Neighborhood, SectionKey};
use crate::render::Immersion;
use glam::DVec3;
use std::collections::HashMap;
use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::{BlockId, Content, Face, surface_sixteenths};
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE, SECTIONS_PER_COLUMN, WORLD_HEIGHT};
use strata_core::light::{ColumnLight, PackedLight, SectionLight, section_index};

#[derive(Debug, Default)]
pub struct ClientWorld {
    /// Shared with the lighting thread, copied only when a block changes.
    columns: HashMap<ColumnPos, Arc<Column>>,
    /// Light received from the lighting thread; open sky where none arrived yet.
    light: HashMap<ColumnPos, ColumnLight>,
    /// Highest non-empty section of each column, kept up to date (asked every frame).
    tops: HashMap<ColumnPos, usize>,
    pub entities: ClientEntities,
}

impl ClientWorld {
    pub fn insert(&mut self, column: Column) {
        let pos = column.pos;
        self.columns.insert(pos, Arc::new(column));
        self.light.remove(&pos);
        self.refresh_top(pos);
    }

    pub fn remove(&mut self, pos: ColumnPos) {
        self.columns.remove(&pos);
        self.light.remove(&pos);
        self.tops.remove(&pos);
    }

    /// The column as shared with the lighting thread.
    pub fn shared_column(&self, pos: ColumnPos) -> Option<Arc<Column>> {
        self.columns.get(&pos).cloned()
    }

    pub fn loaded_columns(&self) -> impl Iterator<Item = &Arc<Column>> {
        self.columns.values()
    }

    /// New light for one section of a loaded column (ignored once the column is gone).
    pub fn set_section_light(&mut self, pos: ColumnPos, index: usize, light: SectionLight) {
        if !self.columns.contains_key(&pos) {
            return;
        }
        let column = self
            .light
            .entry(pos)
            .or_insert_with(|| ColumnLight::below_open_sky(None));
        if let Some(section) = column.section_mut(index) {
            *section = light;
        }
    }

    /// Open sky where no light was computed.
    pub fn light_at(&self, pos: BlockPos) -> PackedLight {
        if !pos.is_in_height_range() {
            return PackedLight::OPEN_SKY;
        }
        let index = (pos.y / SECTION_SIZE) as usize;
        self.light
            .get(&column_of(pos.x, pos.z))
            .and_then(|column| column.section(index))
            .map_or(PackedLight::OPEN_SKY, |section| {
                section.get(section_index(local(pos.x), local(pos.y), local(pos.z)))
            })
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// The column and its eight neighbours are loaded: it can be meshed correctly.
    pub fn is_complete(&self, pos: ColumnPos) -> bool {
        (-1..=1).all(|dz| {
            (-1..=1).all(|dx| {
                self.columns.contains_key(&ColumnPos {
                    x: pos.x + dx,
                    z: pos.z + dz,
                })
            })
        })
    }

    /// Non-empty sections of one column, as mesh keys.
    pub fn column_section_keys(&self, pos: ColumnPos) -> Vec<SectionKey> {
        self.columns
            .get(&pos)
            .map(|column| section_keys_of(column).collect())
            .unwrap_or_default()
    }

    /// Empty sections have no mesh; unloaded ones count as empty.
    pub fn is_section_empty(&self, pos: ColumnPos, index: usize) -> bool {
        self.columns
            .get(&pos)
            .and_then(|column| column.section(index))
            .is_none_or(|section| section.is_empty())
    }

    /// Highest non-empty section of a loaded column.
    pub fn top_section(&self, pos: ColumnPos) -> Option<usize> {
        self.tops.get(&pos).copied()
    }

    fn refresh_top(&mut self, pos: ColumnPos) {
        let top = self
            .columns
            .get(&pos)
            .and_then(|c| c.sections().iter().rposition(|s| !s.is_empty()));
        match top {
            Some(top) => self.tops.insert(pos, top),
            None => self.tops.remove(&pos),
        };
    }

    /// Air where nothing is loaded.
    pub fn block_at(&self, pos: BlockPos) -> BlockId {
        self.columns
            .get(&column_of(pos.x, pos.z))
            .map_or(BlockId::AIR, |column| {
                column.get(local(pos.x), pos.y, local(pos.z))
            })
    }

    /// Whether `position` is inside an opaque block: back faces are culled, so a camera
    /// there sees through the surrounding rock to the void below the world.
    pub fn is_inside_opaque(&self, position: DVec3, table: &BlockTable) -> bool {
        let block = BlockPos {
            x: position.x.floor() as i32,
            y: position.y.floor() as i32,
            z: position.z.floor() as i32,
        };
        let id = self.block_at(block);
        table.is_opaque(id) && table.family(id).is_none()
    }

    /// The liquid `eyes` are in (below its surface), for the fog seen from inside.
    pub fn immersion(&self, eyes: DVec3, content: &Content) -> Option<Immersion> {
        let pos = BlockPos {
            x: eyes.x.floor() as i32,
            y: eyes.y.floor() as i32,
            z: eyes.z.floor() as i32,
        };
        let state = content.blocks.get(self.block_at(pos))?.fluid?;
        let fluid = content.fluids.get(&state.source)?;
        let above = content.blocks.get(self.block_at(pos.neighbour(Face::Up)));
        let full = above
            .and_then(|block| block.fluid)
            .is_some_and(|above| above.source == state.source);
        let surface = if full {
            1.0
        } else {
            f64::from(surface_sixteenths(state.level)) / 16.0
        };
        let [r, g, b, _] = fluid.view.fog_color.0.0;
        (eyes.y - f64::from(pos.y) < surface).then_some(Immersion {
            color: [r, g, b],
            distance: fluid.view.fog_distance,
        })
    }

    /// Whether the column holding `pos` is loaded.
    pub fn is_loaded(&self, pos: BlockPos) -> bool {
        self.columns.contains_key(&column_of(pos.x, pos.z))
    }

    /// Creates the column if needed (used to build scenes on the client).
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        let pos = column_of(x, z);
        let column = self
            .columns
            .entry(pos)
            .or_insert_with(|| Arc::new(Column::empty(pos)));
        Arc::make_mut(column).set(local(x), y, local(z), block);
        self.refresh_top(pos);
    }

    /// True when the section and its six neighbours are all one opaque block: nothing
    /// inside can be seen (deep underground), so meshing is skipped.
    pub fn is_buried(&self, key: SectionKey, table: &BlockTable) -> bool {
        let (cx, cz, index) = key;
        let solid = |x: i32, z: i32, y: isize| {
            if y < 0 || y >= SECTIONS_PER_COLUMN as isize {
                return true;
            }
            self.columns
                .get(&ColumnPos { x, z })
                .and_then(|column| column.section(y as usize))
                .and_then(|section| section.uniform())
                .is_some_and(|block| table.is_opaque(block))
        };
        let y = index as isize;
        solid(cx, cz, y)
            && solid(cx, cz, y - 1)
            && solid(cx, cz, y + 1)
            && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .all(|(dx, dz)| solid(cx + dx, cz + dz, y))
    }

    /// Non-empty sections, as mesh keys.
    pub fn section_keys(&self) -> impl Iterator<Item = SectionKey> + '_ {
        self.columns.values().flat_map(section_keys_of)
    }

    /// Section `key` plus a one-block border, with the 3×3 neighbouring columns looked up once.
    pub fn neighborhood(&self, key: SectionKey, table: &BlockTable) -> Neighborhood {
        let (cx, cz, section) = key;
        let columns = around(&self.columns, cx, cz);
        let lights = around(&self.light, cx, cz);
        let base_y = section as i32 * SECTION_SIZE;
        let blocks = Neighborhood::build(|x, y, z| {
            let (dx, lx) = split(x);
            let (dz, lz) = split(z);
            columns[dz][dx].map_or(BlockId::AIR, |column| column.get(lx, base_y + y, lz))
        })
        .with_light(|x, y, z| {
            let (dx, lx) = split(x);
            let (dz, lz) = split(z);
            let y = base_y + y;
            let Some(section) = lights[dz][dx]
                .filter(|_| (0..WORLD_HEIGHT).contains(&y))
                .and_then(|column| column.section((y / SECTION_SIZE) as usize))
            else {
                return PackedLight::OPEN_SKY;
            };
            section.get(section_index(lx, local(y), lz))
        });
        let palette = columns[1][1]
            .and_then(|column| column.section(section))
            .map_or(&[][..], |s| s.palette());
        let origin = [cx * SECTION_SIZE, base_y, cz * SECTION_SIZE];
        blocks.of_section(origin, palette, table)
    }
}

fn section_keys_of(column: &Arc<Column>) -> impl Iterator<Item = SectionKey> + '_ {
    column
        .sections()
        .iter()
        .enumerate()
        .filter(|(_, section)| !section.is_empty())
        .map(move |(index, _)| (column.pos.x, column.pos.z, index))
}

/// The values of the 3×3 columns centred on `(cx, cz)`, by `[dz][dx]`.
fn around<T>(map: &HashMap<ColumnPos, T>, cx: i32, cz: i32) -> [[Option<&T>; 3]; 3] {
    std::array::from_fn(|dz| {
        std::array::from_fn(|dx| {
            map.get(&ColumnPos {
                x: cx + dx as i32 - 1,
                z: cz + dz as i32 - 1,
            })
        })
    })
}

fn column_of(x: i32, z: i32) -> ColumnPos {
    ColumnPos {
        x: x.div_euclid(SECTION_SIZE),
        z: z.div_euclid(SECTION_SIZE),
    }
}

fn local(c: i32) -> u32 {
    c.rem_euclid(SECTION_SIZE) as u32
}

/// `-1..=16` relative to a section → (neighbour index 0..3, local coordinate).
fn split(c: i32) -> (usize, u32) {
    match c {
        -1 => (0, 15),
        16 => (2, 0),
        _ => (1, c as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighborhood_reads_across_column_borders() {
        let mut world = ClientWorld::default();
        world.set_block(-1, 20, 0, BlockId(3));
        world.set_block(16, 16, 15, BlockId(4));
        let blocks = world.neighborhood((0, 0, 1), &table());
        assert_eq!(blocks.get(-1, 4, 0), BlockId(3));
        assert_eq!(blocks.get(16, 0, 15), BlockId(4));
        assert_eq!(blocks.get(0, 0, 0), BlockId::AIR);
    }

    #[test]
    fn neighborhood_reads_light_across_column_borders() {
        use strata_core::content::Block;
        use strata_core::light::{LightTable, Lighting};
        let mut world = ClientWorld::default();
        for x in -16..32 {
            for z in -16..32 {
                world.set_block(x, 0, z, BlockId(1));
            }
        }
        world.set_block(15, 20, 3, BlockId(2));
        let block = |light_opacity, light_emission| Block {
            light_opacity,
            light_emission,
            ..Block::air()
        };
        let lights = LightTable::new([Block::air(), block(15, 0), block(0, 14)].iter());
        let mut lighting = Lighting::default();
        let positions: Vec<_> = world.columns.keys().copied().collect();
        for &pos in &positions {
            lighting.light_column(&world.columns, &lights, pos);
        }
        for pos in positions {
            let sections = lighting.column(pos).map(|c| c.sections().to_vec());
            for (index, light) in sections.into_iter().flatten().enumerate() {
                world.set_section_light(pos, index, light);
            }
        }
        let blocks = world.neighborhood((1, 0, 1), &table());
        assert_eq!(blocks.light(-1, 4, 3).block(), 14);
        assert_eq!(blocks.light(0, 4, 3).block(), 13);
        assert_eq!(blocks.light(0, 4, 3).sky(), 15);
        assert_eq!(world.light_at(BlockPos::new(16, 20, 4)).block(), 12);
    }

    /// Ids: 0 air, 1 bedrock, 2 deep stone.
    fn table() -> BlockTable {
        use strata_core::content::{Block, RenderKind};
        let solid = Block {
            render: RenderKind::Opaque,
            ..Block::air()
        };
        BlockTable::new([Block::air(), solid.clone(), solid].iter())
    }

    /// Bedrock at y 0, deep stone at y 1-4 and air above, over a 3×3 block of columns.
    fn cavern_floor() -> ClientWorld {
        let mut world = ClientWorld::default();
        for x in -16..32 {
            for z in -16..32 {
                world.set_block(x, 0, z, BlockId(1));
                (1..5).for_each(|y| world.set_block(x, y, z, BlockId(2)));
            }
        }
        world
    }

    #[test]
    fn the_cavern_floor_of_the_lowest_section_is_meshed() {
        let world = cavern_floor();
        let mesh = crate::mesher::mesh_section(
            &world.neighborhood((0, 0, 0), &table()),
            &table(),
            Default::default(),
        );
        let tops = mesh
            .opaque
            .vertices
            .chunks(4)
            .filter(|quad| quad.iter().all(|v| v.position()[1] == 5))
            .count();
        assert_eq!(tops, 1, "one merged 16×16 floor");
        assert_eq!(
            mesh.opaque.quad_count(),
            2,
            "floor and underside of the world"
        );
    }

    #[test]
    fn a_camera_inside_rock_is_detected() {
        let world = cavern_floor();
        assert!(world.is_inside_opaque(DVec3::new(3.5, 2.5, 3.5), &table()));
        assert!(!world.is_inside_opaque(DVec3::new(3.5, 5.5, 3.5), &table()));
    }
}
