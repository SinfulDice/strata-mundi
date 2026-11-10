//! The client's copy of the blocks around the player, as sent by the server.

use crate::entities::ClientEntities;
use crate::mesher::{BlockTable, LightSections, Neighborhood, SectionKey};
use crate::render::Immersion;
use glam::DVec3;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use strata_core::chunk::{BiomeMap, Column};
use strata_core::content::{BiomeTints, BlockId, Content, Face, surface_sixteenths};
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE, SECTIONS_PER_COLUMN};
use strata_core::light::{ColumnLight, PackedLight, SectionLight};
use strata_protocol::{MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE};

mod biomes;

use biomes::TintCache;
mod light;

/// Rings of columns at the edge of the loaded disc that are never drawn: a column is lit
/// once its 8 neighbours are loaded, and meshed once they are all lit (D046).
pub const UNDRAWN_RING: u32 = 2;

/// Columns to ask the server for, so that `render_distance` columns are drawn.
pub fn requested_view_distance(render_distance: u32) -> u32 {
    (render_distance + UNDRAWN_RING).clamp(MIN_VIEW_DISTANCE, MAX_VIEW_DISTANCE)
}

/// Columns drawn around the player for a render distance (less only at the protocol's
/// bound).
pub fn drawn_distance(render_distance: u32) -> u32 {
    requested_view_distance(render_distance).saturating_sub(UNDRAWN_RING)
}

#[derive(Debug, Default)]
pub struct ClientWorld {
    /// Shared with the lighting thread, copied only when a block changes.
    columns: HashMap<ColumnPos, Arc<Column>>,
    biomes: HashMap<ColumnPos, Arc<BiomeMap>>,
    /// Light of the lit columns, from the lighting thread (or computed for a capture).
    light: HashMap<ColumnPos, ColumnLight>,
    /// Highest non-empty section of each column, kept up to date (asked every frame).
    tops: HashMap<ColumnPos, usize>,
    pub entities: ClientEntities,
    /// Colours of the server's biomes, by `BiomeId`.
    biome_tints: Arc<[BiomeTints]>,
    tints: RefCell<TintCache>,
}

impl ClientWorld {
    pub fn insert(&mut self, column: Column, biomes: BiomeMap) {
        let pos = column.pos;
        self.columns.insert(pos, Arc::new(column));
        self.biomes.insert(pos, Arc::new(biomes));
        self.forget_tints_around(pos);
        self.light.remove(&pos);
        self.refresh_top(pos);
    }

    pub fn remove(&mut self, pos: ColumnPos) {
        self.columns.remove(&pos);
        self.biomes.remove(&pos);
        self.tints.get_mut().forget(pos);
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

    /// Section `key` plus a one-block border, with the 3×3 neighbouring columns looked up
    /// once, and biome colours blended over `blend` blocks.
    pub fn neighborhood(&self, key: SectionKey, table: &BlockTable, blend: u32) -> Neighborhood {
        let (cx, cz, section) = key;
        let columns = around(&self.columns, cx, cz);
        let base_y = section as i32 * SECTION_SIZE;
        let blocks = Neighborhood::build(|x, y, z| {
            let (dx, lx) = split(x);
            let (dz, lz) = split(z);
            columns[dz][dx].map_or(BlockId::AIR, |column| column.get(lx, base_y + y, lz))
        })
        .with_light_sections(self.light_sections(cx, cz, section));
        let palette = columns[1][1]
            .and_then(|column| column.section(section))
            .map_or(&[][..], |s| s.palette());
        let origin = [cx * SECTION_SIZE, base_y, cz * SECTION_SIZE];
        let blocks = blocks.of_section(origin, palette, table);
        match self
            .column_tints(cx, cz, blend)
            .filter(|_| blocks.contents().tinted)
        {
            Some(tints) => blocks.with_tints(tints),
            None => blocks,
        }
    }
}

impl ClientWorld {
    /// Shared copies of the light around a section; open sky outside the world height and
    /// in unlit columns.
    fn light_sections(&self, cx: i32, cz: i32, section: usize) -> LightSections {
        let lights = around(&self.light, cx, cz);
        Box::new(std::array::from_fn(|dy| {
            std::array::from_fn(|dz| {
                std::array::from_fn(|dx| {
                    (section + dy)
                        .checked_sub(1)
                        .and_then(|index| lights[dz][dx]?.section(index))
                        .cloned()
                        .unwrap_or(SectionLight::Uniform(PackedLight::OPEN_SKY))
                })
            })
        }))
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

pub(crate) fn column_of(x: i32, z: i32) -> ColumnPos {
    ColumnPos {
        x: x.div_euclid(SECTION_SIZE),
        z: z.div_euclid(SECTION_SIZE),
    }
}

pub(crate) fn local(c: i32) -> u32 {
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
pub(crate) mod tests {
    use super::*;

    #[test]
    fn neighborhood_reads_across_column_borders() {
        let mut world = ClientWorld::default();
        world.set_block(-1, 20, 0, BlockId(3));
        world.set_block(16, 16, 15, BlockId(4));
        let blocks = world.neighborhood((0, 0, 1), &table(), 0);
        assert_eq!(blocks.get(-1, 4, 0), BlockId(3));
        assert_eq!(blocks.get(16, 0, 15), BlockId(4));
        assert_eq!(blocks.get(0, 0, 0), BlockId::AIR);
    }

    /// Ids: 0 air, 1 bedrock, 2 deep stone.
    pub(super) fn table() -> BlockTable {
        use strata_core::content::{Block, RenderKind};
        let solid = Block {
            render: RenderKind::Opaque,
            ..Block::air()
        };
        BlockTable::new([Block::air(), solid.clone(), solid].iter())
    }

    /// Bedrock at y 0, deep stone at y 1-4 and air above, over a 3×3 block of columns.
    pub(super) fn cavern_floor() -> ClientWorld {
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
            &world.neighborhood((0, 0, 0), &table(), 0),
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
