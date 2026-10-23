//! Applies the columns streamed by the server to the client world and schedules meshing.
//! A column is meshed once its eight neighbours are present, so its border faces and
//! corner shading are right the first time and never need redoing.

use crate::mesh_scheduler::MeshScheduler;
use crate::render::{Gpu, Renderer};
use crate::world::ClientWorld;
use glam::DVec3;
use strata_core::chunk::Column;
use strata_core::coords::ColumnPos;

pub struct WorldView {
    pub world: ClientWorld,
    meshing: MeshScheduler,
}

impl WorldView {
    pub fn new(world: ClientWorld, mut meshing: MeshScheduler) -> Self {
        for key in world.section_keys() {
            meshing.mark_dirty(key);
        }
        Self { world, meshing }
    }

    pub fn insert(&mut self, column: Column) {
        let pos = column.pos;
        self.world.insert(column);
        for neighbour in around(pos).chain(std::iter::once(pos)) {
            if self.world.is_complete(neighbour) {
                for key in self.world.column_section_keys(neighbour) {
                    self.meshing.mark_dirty(key);
                }
            }
        }
    }

    pub fn remove(&mut self, pos: ColumnPos, renderer: &mut Renderer) {
        self.world.remove(pos);
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
        self.meshing
            .update(&self.world, gpu, renderer, camera, vertical_distance);
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
