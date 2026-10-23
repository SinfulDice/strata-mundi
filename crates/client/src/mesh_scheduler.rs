use crate::mesher::{BlockTable, MeshJob, MeshWorkers, SectionKey, SectionMesh};
use crate::render::{Gpu, Renderer, in_vertical_range};
use crate::world::ClientWorld;
use glam::DVec3;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use strata_core::coords::{ColumnPos, SECTION_SIZE, SECTIONS_PER_COLUMN};

/// Time the main thread may spend uploading finished meshes each frame.
const UPLOAD_BUDGET: Duration = Duration::from_millis(4);
/// Jobs queued or being meshed per worker thread.
const JOBS_PER_WORKER: usize = 4;

/// Decides which sections to (re)mesh, in which order, and uploads the results.
pub struct MeshScheduler {
    workers: MeshWorkers,
    table: Arc<BlockTable>,
    max_in_flight: usize,
    dirty: HashSet<SectionKey>,
    /// Latest request per section. Versions come from one global counter, so a section
    /// that is forgotten then reloaded can never accept a result from before.
    versions: HashMap<SectionKey, u64>,
    next_version: u64,
    in_flight: HashSet<SectionKey>,
    /// Dirty sections within the vertical distance that could not be submitted yet.
    waiting: usize,
}

impl MeshScheduler {
    pub fn new(
        table: Arc<BlockTable>,
        threads: usize,
        ambient_occlusion: bool,
    ) -> std::io::Result<Self> {
        let max_in_flight = threads.max(1) * JOBS_PER_WORKER;
        Ok(Self {
            workers: MeshWorkers::spawn(
                threads,
                Arc::clone(&table),
                max_in_flight,
                ambient_occlusion,
            )?,
            table,
            max_in_flight,
            dirty: HashSet::new(),
            versions: HashMap::new(),
            next_version: 0,
            in_flight: HashSet::new(),
            waiting: 0,
        })
    }

    pub fn mark_dirty(&mut self, key: SectionKey) {
        self.next_version += 1;
        self.versions.insert(key, self.next_version);
        self.dirty.insert(key);
    }

    /// Stops tracking a column that left the view distance.
    pub fn forget_column(&mut self, pos: ColumnPos) {
        for index in 0..SECTIONS_PER_COLUMN as usize {
            let key = (pos.x, pos.z, index);
            self.dirty.remove(&key);
            self.versions.remove(&key);
        }
    }

    /// Visible sections still waiting to be meshed or uploaded (sections beyond the
    /// vertical distance wait without counting).
    pub fn pending(&self) -> usize {
        self.waiting + self.in_flight.len()
    }

    /// Submits the nearest dirty sections within the vertical distance (see `in_vertical_range`),
    /// then uploads finished meshes within the frame budget.
    pub fn update(
        &mut self,
        world: &ClientWorld,
        gpu: &Gpu,
        renderer: &mut Renderer,
        camera: DVec3,
        vertical_distance: u32,
    ) {
        self.submit_nearest(world, gpu, renderer, camera, vertical_distance);
        let start = Instant::now();
        for result in self.workers.finished() {
            self.in_flight.remove(&result.key);
            if self.versions.get(&result.key) == Some(&result.version) {
                renderer.upload_section(gpu, result.key, &result.mesh);
            }
            if start.elapsed() > UPLOAD_BUDGET {
                break;
            }
        }
    }

    fn submit_nearest(
        &mut self,
        world: &ClientWorld,
        gpu: &Gpu,
        renderer: &mut Renderer,
        camera: DVec3,
        vertical_distance: u32,
    ) {
        let camera_section = (camera.y / f64::from(SECTION_SIZE)).floor() as i64;
        let eligible = |key: &SectionKey| {
            let top = world.top_section(ColumnPos { x: key.0, z: key.1 });
            !self.in_flight.contains(key)
                && top.is_some_and(|top| {
                    in_vertical_range(key.2, top, camera_section, vertical_distance)
                })
        };
        let room = self.max_in_flight.saturating_sub(self.in_flight.len());
        if room == 0 {
            self.waiting = self.dirty.iter().filter(|key| eligible(key)).count();
            return;
        }
        let mut candidates: Vec<SectionKey> = self.dirty.iter().copied().filter(eligible).collect();
        self.waiting = candidates.len();
        candidates.sort_by_cached_key(|key| distance_key(*key, camera));
        for key in candidates.into_iter().take(room) {
            self.waiting -= 1;
            self.dirty.remove(&key);
            if world.is_buried(key, &self.table) {
                renderer.upload_section(gpu, key, &SectionMesh::default());
                continue;
            }
            let job = MeshJob {
                key,
                version: self.versions.get(&key).copied().unwrap_or_default(),
                blocks: world.neighborhood(key),
            };
            if self.workers.submit(job).is_err() {
                self.dirty.insert(key);
                self.waiting += 1;
                break;
            }
            self.in_flight.insert(key);
        }
    }
}

/// Squared distance from the camera to the section centre, as an integer sort key.
fn distance_key(key: SectionKey, camera: DVec3) -> u64 {
    let size = f64::from(SECTION_SIZE);
    let center = DVec3::new(
        (f64::from(key.0) + 0.5) * size,
        (key.2 as f64 + 0.5) * size,
        (f64::from(key.1) + 0.5) * size,
    );
    center.distance_squared(camera) as u64
}
