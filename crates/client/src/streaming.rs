//! Applies the columns streamed by the server to the client world and schedules meshing.
//! A column is meshed once its eight neighbours are present, so its border faces and
//! corner shading are right the first time and never need redoing.

use crate::lighting::LightWorker;
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
    pub fn new(world: ClientWorld, mut meshing: MeshScheduler, lighting: LightWorker) -> Self {
        for key in world.section_keys() {
            meshing.mark_dirty(key);
        }
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
            if self.world.is_complete(ColumnPos { x: key.0, z: key.1 }) {
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
        for neighbour in around(pos).chain(std::iter::once(pos)) {
            if self.world.is_complete(neighbour) {
                for key in self.world.column_section_keys(neighbour) {
                    self.meshing.mark_dirty(key);
                }
            }
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
            if self.world.is_complete(ColumnPos { x: key.0, z: key.1 }) {
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
    /// finished; sections with new light are meshed again.
    fn exchange_light(&mut self) {
        for (pos, changes) in self.light_changes.drain() {
            if let Some(column) = self.world.shared_column(pos) {
                self.lighting.changed(column, changes);
            }
        }
        for result in self.lighting.finished() {
            for (pos, index, light) in result.sections {
                self.world.set_section_light(pos, index, light);
                if self.world.is_complete(pos) && !self.world.is_section_empty(pos, index) {
                    self.meshing.mark_dirty((pos.x, pos.z, index));
                }
            }
        }
    }

    pub fn pending_meshes(&self) -> usize {
        self.meshing.pending()
    }

    /// Nothing left to mesh among at least `columns` loaded columns.
    pub fn is_settled(&self, columns: usize) -> bool {
        self.world.column_count() >= columns && self.meshing.pending() == 0
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
