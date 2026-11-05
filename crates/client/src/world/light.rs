//! Light of the client world: received from the lighting thread, or computed at once for
//! captures. A section is meshed only once its column and the eight around are lit, so
//! its smooth light (border included) is right the first time.

use super::{ClientWorld, column_of, local};
use crate::models::ModelPose;
use glam::DVec3;
use strata_core::content::{EntityTypeId, Face};
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE};
use strata_core::light::section_index;
use strata_core::light::{
    Channel, ColumnLight, LightTable, Lighting, PackedLight, SectionLight, attenuate,
};

/// Height above an entity's position where its light is read (above the water a boat floats in).
const ENTITY_LIGHT_HEIGHT: f64 = 0.5;

impl ClientWorld {
    /// The lighting thread lit a loaded column: its sections follow (open sky until then).
    pub fn mark_lit(&mut self, pos: ColumnPos) {
        if self.columns.contains_key(&pos) {
            self.light
                .entry(pos)
                .or_insert_with(|| ColumnLight::below_open_sky(None));
        }
    }

    /// Light off, or the lighting thread gone: the column is lit like the open sky.
    pub fn set_flat_light(&mut self, pos: ColumnPos) {
        if self.columns.contains_key(&pos) {
            self.light.insert(pos, ColumnLight::below_open_sky(None));
        }
    }

    /// Every loaded column lit like the open sky.
    pub fn light_flat(&mut self) {
        let loaded: Vec<ColumnPos> = self.columns.keys().copied().collect();
        loaded.into_iter().for_each(|pos| self.set_flat_light(pos));
    }

    /// Every column back to unlit, waiting for the lighting thread.
    pub fn clear_light(&mut self) {
        self.light.clear();
    }

    pub fn is_lit(&self, pos: ColumnPos) -> bool {
        self.light.contains_key(&pos)
    }

    /// The column and its eight neighbours are lit: its sections can be meshed.
    pub fn is_light_complete(&self, pos: ColumnPos) -> bool {
        (-1..=1).all(|dz| {
            (-1..=1).all(|dx| {
                self.is_lit(ColumnPos {
                    x: pos.x + dx,
                    z: pos.z + dz,
                })
            })
        })
    }

    /// New light for one section of a lit column (ignored for any other).
    pub fn set_section_light(&mut self, pos: ColumnPos, index: usize, light: SectionLight) {
        if let Some(section) = self
            .light
            .get_mut(&pos)
            .and_then(|column| column.section_mut(index))
        {
            *section = light;
        }
    }

    /// Open sky where no light was computed.
    pub fn light_at(&self, pos: BlockPos) -> PackedLight {
        self.computed_light(pos).unwrap_or(PackedLight::OPEN_SKY)
    }

    fn computed_light(&self, pos: BlockPos) -> Option<PackedLight> {
        if !pos.is_in_height_range() {
            return None;
        }
        let index = (pos.y / SECTION_SIZE) as usize;
        let section = self.light.get(&column_of(pos.x, pos.z))?.section(index)?;
        Some(section.get(section_index(local(pos.x), local(pos.y), local(pos.z))))
    }

    /// Until the lighting thread answers, a changed cell takes the light its lit neighbours
    /// would give it (and its own glow), so the faces it reveals are not drawn dark. It
    /// only brightens: the thread's answer replaces it.
    pub fn guess_light(&mut self, pos: BlockPos, table: &LightTable) {
        let Some(current) = self.computed_light(pos) else {
            return;
        };
        let block = self.block_at(pos);
        let mut guess = current;
        for channel in Channel::ALL {
            let around = Face::ALL.iter().filter_map(|&face| {
                let near = self.computed_light(pos.neighbour(face))?;
                Some(attenuate(
                    channel,
                    near.get(channel),
                    face.opposite(),
                    table.opacity(block),
                ))
            });
            let own = match channel {
                Channel::Block => table.emission(block),
                Channel::Sky => 0,
            };
            let level = around.chain([own]).max().unwrap_or(0);
            if level > guess.get(channel) {
                guess = guess.with(channel, level);
            }
        }
        if guess != current
            && let Some(section) = self
                .light
                .get_mut(&column_of(pos.x, pos.z))
                .and_then(|column| column.section_mut((pos.y / SECTION_SIZE) as usize))
        {
            section.set(
                section_index(local(pos.x), local(pos.y), local(pos.z)),
                guess,
            );
        }
    }

    /// Lights every loaded column at once, for captures (which mesh right after). Light
    /// stops at the edge of the loaded area, as the result does not depend on the order.
    pub fn compute_light(&mut self, table: &LightTable) {
        let mut lighting = Lighting::default();
        let loaded: Vec<ColumnPos> = self.columns.keys().copied().collect();
        for &pos in &loaded {
            lighting.light_column(&self.columns, table, pos);
        }
        for pos in loaded {
            if let Some(light) = lighting.column(pos) {
                self.light.insert(pos, light.clone());
            }
        }
    }

    /// Entities to draw, with the light of their cell.
    pub fn entity_models(
        &self,
    ) -> impl Iterator<Item = (EntityTypeId, ModelPose, PackedLight)> + '_ {
        self.entities.iter().map(|(_, kind, pose)| {
            let at = pose.position + DVec3::Y * ENTITY_LIGHT_HEIGHT;
            let cell = BlockPos {
                x: at.x.floor() as i32,
                y: at.y.floor() as i32,
                z: at.z.floor() as i32,
            };
            (kind, pose, self.light_at(cell))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::table;
    use super::*;
    use strata_core::content::{Block, BlockId};

    fn light_table() -> LightTable {
        let block = |light_opacity, light_emission| Block {
            light_opacity,
            light_emission,
            ..Block::air()
        };
        LightTable::new([Block::air(), block(15, 0), block(0, 14)].iter())
    }

    #[test]
    fn neighborhood_reads_light_across_column_borders() {
        let mut world = ClientWorld::default();
        for x in -16..32 {
            for z in -16..32 {
                world.set_block(x, 0, z, BlockId(1));
            }
        }
        world.set_block(15, 20, 3, BlockId(2));
        world.compute_light(&light_table());
        let blocks = world.neighborhood((1, 0, 1), &table());
        assert_eq!(blocks.light(-1, 4, 3).block(), 14);
        assert_eq!(blocks.light(0, 4, 3).block(), 13);
        assert_eq!(blocks.light(0, 4, 3).sky(), 15);
        assert_eq!(world.light_at(BlockPos::new(16, 20, 4)).block(), 12);
        let mut resolved = blocks.clone();
        resolved.resolve_light();
        for (x, y, z) in [(-1, 4, 3), (0, 4, 3), (16, -1, 16), (7, 16, -1), (3, 3, 3)] {
            assert_eq!(resolved.light(x, y, z), blocks.light(x, y, z));
        }
    }

    #[test]
    fn a_broken_block_takes_the_light_of_its_neighbours_until_the_thread_answers() {
        let mut world = ClientWorld::default();
        for x in -16..32 {
            for z in -16..32 {
                (0..=10).for_each(|y| world.set_block(x, y, z, BlockId(1)));
            }
        }
        world.compute_light(&light_table());
        let broken = BlockPos::new(5, 10, 5);
        assert_eq!(world.light_at(broken), PackedLight::DARK);
        world.set_block(broken.x, broken.y, broken.z, BlockId::AIR);
        world.guess_light(broken, &light_table());
        assert_eq!(
            world.light_at(broken),
            PackedLight::OPEN_SKY,
            "full sky from above"
        );
        let below = BlockPos::new(5, 9, 5);
        world.set_block(below.x, below.y, below.z, BlockId(2));
        world.guess_light(below, &light_table());
        assert_eq!(
            world.light_at(below),
            PackedLight::new(15, 14),
            "sky and its own glow"
        );
        let deep = BlockPos::new(5, 3, 5);
        world.set_block(deep.x, deep.y, deep.z, BlockId::AIR);
        world.guess_light(deep, &light_table());
        assert_eq!(
            world.light_at(deep),
            PackedLight::DARK,
            "nothing lit around"
        );
    }

    #[test]
    fn a_column_is_light_complete_once_it_and_its_neighbours_are_lit() {
        let mut world = ClientWorld::default();
        for x in -32..48 {
            for z in -32..48 {
                world.set_block(x, 0, z, BlockId(1));
            }
        }
        let centre = ColumnPos { x: 0, z: 0 };
        world.mark_lit(centre);
        assert!(!world.is_light_complete(centre));
        for dz in -1..=1 {
            for dx in -1..=1 {
                world.mark_lit(ColumnPos { x: dx, z: dz });
            }
        }
        assert!(world.is_light_complete(centre));
        assert!(!world.is_light_complete(ColumnPos { x: 1, z: 0 }));
        world.mark_lit(ColumnPos { x: 9, z: 9 });
        assert!(!world.is_lit(ColumnPos { x: 9, z: 9 }), "not loaded");
    }
}
