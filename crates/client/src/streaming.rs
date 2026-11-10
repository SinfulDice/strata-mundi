//! Applies the columns streamed by the server to the client world and schedules meshing.
//! A column is meshed once it and its eight neighbours are lit (which needs their own
//! neighbours loaded), so its border faces, corner shading and smooth light are right the
//! first time; afterwards only the sections the light engine reports are meshed again.

use crate::lighting::{LightResult, LightWorker};
use crate::mesh_scheduler::MeshScheduler;
use crate::mesher::SectionKey;
use crate::particles::ParticleSystem;
use crate::render::{Gpu, Renderer};
use crate::settings::Lighting;
use crate::sky::BiomeFog;
use crate::world::ClientWorld;
use glam::{DVec3, Vec3};
use std::collections::HashMap;
use std::sync::Arc;
use strata_core::chunk::{BiomeMap, Column};
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE, WORLD_HEIGHT};
use strata_core::light::LightTable;

pub struct WorldView {
    pub world: ClientWorld,
    meshing: MeshScheduler,
    /// `None` when light is off (flat) or its thread failed.
    lighting: Option<LightWorker>,
    light_table: LightTable,
    /// Block changes of this frame, by column, with the previous block: sent to the
    /// lighting thread together, so a column is copied at most once per frame.
    light_changes: HashMap<ColumnPos, Vec<(BlockPos, BlockId)>>,
    biome_fog: BiomeFog,
    /// Off until given one (`with_particles`).
    pub particles: ParticleSystem,
    /// Block changes of this frame with the previous block, for the sounds.
    pub block_changes: Vec<(BlockPos, BlockId, BlockId)>,
}

impl WorldView {
    /// Prebuilt worlds (title panorama) are meshed as their light arrives.
    pub fn new(
        world: ClientWorld,
        meshing: MeshScheduler,
        light_table: LightTable,
        mode: Lighting,
    ) -> std::io::Result<Self> {
        let mut view = Self {
            world,
            meshing,
            lighting: None,
            light_table,
            light_changes: HashMap::new(),
            biome_fog: BiomeFog::default(),
            particles: ParticleSystem::default(),
            block_changes: Vec::new(),
        };
        match mode {
            Lighting::Full => view.start_lighting()?,
            Lighting::Flat => view.light_flat(),
        }
        Ok(view)
    }

    pub fn with_particles(mut self, particles: ParticleSystem) -> Self {
        self.particles = particles;
        self
    }

    /// Switches light on or off while playing; the old meshes stay on screen until
    /// their replacement arrives.
    pub fn set_lighting(&mut self, mode: Lighting) {
        match mode {
            Lighting::Full if self.lighting.is_none() => {
                if let Err(error) = self.start_lighting() {
                    tracing::error!(%error, "could not start the lighting thread");
                    self.light_flat();
                }
            }
            Lighting::Flat if self.lighting.is_some() => {
                self.lighting = None;
                self.light_flat();
            }
            _ => {}
        }
    }

    fn start_lighting(&mut self) -> std::io::Result<()> {
        let mut lighting = LightWorker::spawn(self.light_table.clone())?;
        self.world.clear_light();
        self.light_changes.clear();
        for column in self.world.loaded_columns() {
            lighting.insert(Arc::clone(column));
        }
        self.lighting = Some(lighting);
        Ok(())
    }

    /// Every column lit like the open sky at once, and meshed again.
    fn light_flat(&mut self) {
        self.world.light_flat();
        self.light_changes.clear();
        self.remesh_all();
    }

    /// New meshing settings (ambient occlusion, threads): every section is meshed again;
    /// the old meshes stay on screen until their replacement arrives.
    pub fn replace_meshing(&mut self, meshing: MeshScheduler) {
        self.meshing = meshing;
        self.remesh_all();
    }

    fn remesh_all(&mut self) {
        for key in self.world.section_keys() {
            if self
                .world
                .is_light_complete(ColumnPos { x: key.0, z: key.1 })
            {
                self.meshing.mark_dirty(key);
            }
        }
    }

    /// Colour factor of the fog around `camera`, eased over the frames.
    pub fn follow_biome_fog(&mut self, camera: DVec3, seconds: f32) -> Vec3 {
        let target = self.world.fog_tint_at(camera.x, camera.z);
        self.biome_fog.follow(target, seconds)
    }

    pub fn insert(&mut self, column: Column, biomes: BiomeMap) {
        let pos = column.pos;
        self.world.insert(column, biomes);
        self.light_changes.remove(&pos);
        match (&mut self.lighting, self.world.shared_column(pos)) {
            (Some(lighting), Some(shared)) => lighting.insert(shared),
            (None, _) => {
                self.world.set_flat_light(pos);
                self.apply_light(LightResult {
                    lit: vec![pos],
                    ..LightResult::default()
                });
            }
            (Some(_), None) => {}
        }
    }

    /// A block changed (server decision): every section whose mesh can see it is redone,
    /// including neighbours across section borders (faces and corner shading), with a
    /// guessed light for the cell until the lighting thread answers.
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) {
        if !self.world.is_loaded(pos) {
            return;
        }
        let old = self.world.block_at(pos);
        self.particles.block_changed(pos, old, block, &self.world);
        self.block_changes.push((pos, old, block));
        self.world.set_block(pos.x, pos.y, pos.z, block);
        if self.lighting.is_some() {
            self.world.guess_light(pos, &self.light_table);
            self.light_changes
                .entry(pos.column())
                .or_default()
                .push((pos, old));
        }
        for key in sections_seeing(pos) {
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
        if let Some(lighting) = &mut self.lighting {
            lighting.remove(pos);
        }
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
            .update(&mut self.world, gpu, renderer, camera, vertical_distance);
    }

    /// Sends this frame's block changes to the lighting thread and takes the light it
    /// finished.
    fn exchange_light(&mut self) {
        let Some(lighting) = &mut self.lighting else {
            return;
        };
        for (pos, changes) in self.light_changes.drain() {
            if let Some(column) = self.world.shared_column(pos) {
                lighting.changed(column, changes);
            }
        }
        let results = lighting.finished();
        let failed = lighting.has_failed();
        for result in results {
            self.apply_light(result);
        }
        if failed {
            self.lighting = None;
            self.light_flat();
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
            && self.lighting.as_ref().is_none_or(LightWorker::is_idle)
            && self.meshing.pending() == 0
    }
}

/// Sections whose mesh shows the block at `pos` (faces and corner shading).
fn sections_seeing(pos: BlockPos) -> Vec<SectionKey> {
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
    keys
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
mod tests;
