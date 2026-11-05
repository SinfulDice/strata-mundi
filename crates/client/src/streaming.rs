//! Applies the columns streamed by the server to the client world and schedules meshing.
//! A column is meshed once it and its eight neighbours are lit (which needs their own
//! neighbours loaded), so its border faces, corner shading and smooth light are right the
//! first time; afterwards only the sections the light engine reports are meshed again.

use crate::lighting::{LightResult, LightWorker};
use crate::mesh_scheduler::MeshScheduler;
use crate::render::{Gpu, Renderer};
use crate::world::ClientWorld;
use glam::DVec3;
use std::collections::HashMap;
use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE, WORLD_HEIGHT};

pub struct WorldView {
    pub world: ClientWorld,
    meshing: MeshScheduler,
    lighting: LightWorker,
    /// Block changes of this frame, by column, with the previous block: sent to the
    /// lighting thread together, so a column is copied at most once per frame.
    light_changes: HashMap<ColumnPos, Vec<(BlockPos, BlockId)>>,
}

impl WorldView {
    /// Prebuilt worlds (title panorama) are meshed as their light arrives.
    pub fn new(world: ClientWorld, meshing: MeshScheduler, mut lighting: LightWorker) -> Self {
        for column in world.loaded_columns() {
            lighting.insert(Arc::clone(column));
        }
        Self {
            world,
            meshing,
            lighting,
            light_changes: HashMap::new(),
        }
    }

    /// New meshing settings (ambient occlusion, threads): every section is meshed again;
    /// the old meshes stay on screen until their replacement arrives.
    pub fn replace_meshing(&mut self, meshing: MeshScheduler) {
        self.meshing = meshing;
        for key in self.world.section_keys() {
            if self
                .world
                .is_light_complete(ColumnPos { x: key.0, z: key.1 })
            {
                self.meshing.mark_dirty(key);
            }
        }
    }

    pub fn insert(&mut self, column: Column) {
        let pos = column.pos;
        self.world.insert(column);
        self.light_changes.remove(&pos);
        if let Some(shared) = self.world.shared_column(pos) {
            self.lighting.insert(shared);
        }
    }

    /// A block changed (server decision): every section whose mesh can see it is redone,
    /// including neighbours across section borders (faces and corner shading).
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) {
        if !self.world.is_loaded(pos) {
            return;
        }
        let old = self.world.block_at(pos);
        self.world.set_block(pos.x, pos.y, pos.z, block);
        self.light_changes
            .entry(pos.column())
            .or_default()
            .push((pos, old));
        let mut keys = Vec::with_capacity(8);
        for dy in -1..=1 {
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let (x, y, z) = (pos.x + dx, pos.y + dy, pos.z + dz);
                    if !(0..WORLD_HEIGHT).contains(&y) {
                        continue;
                    }
                    let key = (
                        x.div_euclid(SECTION_SIZE),
                        z.div_euclid(SECTION_SIZE),
                        (y / SECTION_SIZE) as usize,
                    );
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
            }
        }
        for key in keys {
            if self
                .world
                .is_light_complete(ColumnPos { x: key.0, z: key.1 })
            {
                self.meshing.mark_dirty(key);
            }
        }
    }

    pub fn remove(&mut self, pos: ColumnPos, renderer: &mut Renderer) {
        self.world.remove(pos);
        self.light_changes.remove(&pos);
        self.lighting.remove(pos);
        self.meshing.forget_column(pos);
        renderer.remove_column(pos.x, pos.z);
    }

    pub fn update(
        &mut self,
        gpu: &Gpu,
        renderer: &mut Renderer,
        camera: DVec3,
        vertical_distance: u32,
    ) {
        self.exchange_light();
        self.meshing
            .update(&self.world, gpu, renderer, camera, vertical_distance);
    }

    /// Sends this frame's block changes to the lighting thread and takes the light it
    /// finished.
    fn exchange_light(&mut self) {
        for (pos, changes) in self.light_changes.drain() {
            if let Some(column) = self.world.shared_column(pos) {
                self.lighting.changed(column, changes);
            }
        }
        for result in self.lighting.finished() {
            self.apply_light(result);
        }
    }

    /// Columns whose light (and neighbours' light) just became complete are meshed for the
    /// first time; in the others, only the sections the engine reports are meshed again.
    fn apply_light(&mut self, result: LightResult) {
        for &pos in &result.lit {
            self.world.mark_lit(pos);
        }
        for (pos, index, light) in result.sections {
            self.world.set_section_light(pos, index, light);
            if self.world.is_light_complete(pos) && !self.world.is_section_empty(pos, index) {
                self.meshing.mark_dirty((pos.x, pos.z, index));
            }
        }
        for &lit in &result.lit {
            for pos in around(lit).chain(std::iter::once(lit)) {
                if self.world.is_light_complete(pos) {
                    for key in self.world.column_section_keys(pos) {
                        self.meshing.mark_dirty(key);
                    }
                }
            }
        }
    }

    pub fn pending_meshes(&self) -> usize {
        self.meshing.pending()
    }

    /// Nothing left to light or mesh among at least `columns` loaded columns.
    pub fn is_settled(&self, columns: usize) -> bool {
        self.world.column_count() >= columns
            && self.lighting.is_idle()
            && self.meshing.pending() == 0
    }
}

fn around(pos: ColumnPos) -> impl Iterator<Item = ColumnPos> {
    (-1..=1)
        .flat_map(|dz| (-1..=1).map(move |dx| (dx, dz)))
        .filter(|&d| d != (0, 0))
        .map(move |(dx, dz)| ColumnPos {
            x: pos.x + dx,
            z: pos.z + dz,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesher::{BlockTable, MeshOptions};
    use std::collections::HashSet;
    use strata_core::content::{Block, RenderKind};
    use strata_core::light::{LightTable, PackedLight, SectionLight};

    fn view() -> WorldView {
        let solid = Block {
            render: RenderKind::Opaque,
            ..Block::air()
        };
        let table = Arc::new(BlockTable::new([Block::air(), solid].iter()));
        let meshing = MeshScheduler::new(table, 1, MeshOptions::default()).unwrap();
        let lighting = LightWorker::spawn(LightTable::default()).unwrap();
        WorldView::new(ClientWorld::default(), meshing, lighting)
    }

    /// 5×5 columns, each with blocks in sections 0 and 2.
    fn loaded() -> WorldView {
        let mut view = view();
        for cz in -2..=2 {
            for cx in -2..=2 {
                let pos = ColumnPos { x: cx, z: cz };
                let mut column = Column::empty(pos);
                column.set(0, 0, 0, BlockId(1));
                column.set(0, 40, 0, BlockId(1));
                view.insert(column);
            }
        }
        view
    }

    fn lit(positions: impl IntoIterator<Item = ColumnPos>) -> LightResult {
        LightResult {
            lit: positions.into_iter().collect(),
            ..LightResult::default()
        }
    }

    const CENTRE: ColumnPos = ColumnPos { x: 0, z: 0 };

    #[test]
    fn a_section_waits_for_the_light_of_its_column_and_neighbours() {
        let mut view = loaded();
        assert!(view.meshing.take_dirty().is_empty(), "nothing lit yet");
        view.apply_light(lit(around(CENTRE)));
        assert!(
            view.meshing.take_dirty().is_empty(),
            "the centre is not lit"
        );
        view.apply_light(lit([CENTRE]));
        let dirty = view.meshing.take_dirty();
        let expected: HashSet<_> = [(0, 0, 0), (0, 0, 2)].into();
        assert_eq!(dirty, expected, "only the centre has its 8 neighbours lit");
    }

    #[test]
    fn a_light_change_meshes_exactly_the_reported_sections() {
        let mut view = loaded();
        view.apply_light(lit(around(CENTRE).chain([CENTRE])));
        view.meshing.take_dirty();
        let light = SectionLight::Uniform(PackedLight::new(3, 9));
        let sections = vec![
            (CENTRE, 2, light.clone()),
            (CENTRE, 5, light.clone()),
            (ColumnPos { x: 1, z: 0 }, 0, light.clone()),
        ];
        view.apply_light(LightResult {
            sections,
            ..LightResult::default()
        });
        let expected: HashSet<_> = [(0, 0, 2)].into();
        assert_eq!(
            view.meshing.take_dirty(),
            expected,
            "empty sections and columns without lit neighbours wait"
        );
        let cell = BlockPos::new(3, 35, 3);
        assert_eq!(view.world.light_at(cell), PackedLight::new(3, 9));
    }
}
