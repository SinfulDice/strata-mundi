//! Keeps the columns around the player loaded: nearest and in front first, generated in
//! the background, sent to the client, and forgotten once far behind.

use crate::generation::{Generated, GenerationWorkers};
use crate::save::{RegionStore, SaveError};
use std::collections::{HashMap, HashSet};
use strata_core::chunk::{BiomeMap, Column};
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE};
use strata_core::updates::BlockAccess;

/// Columns sent per tick at most, so a burst never floods the client.
const SENT_PER_TICK: usize = 48;
/// Columns are dropped only this far beyond the view distance, so moving back and forth
/// along a border does not reload them.
const UNLOAD_MARGIN: i32 = 2;
/// Saved columns decompressed per tick at most, so the tick never stalls.
const SAVED_PER_TICK: usize = 16;

/// Where the player is, as last reported.
#[derive(Debug, Clone, Copy)]
pub struct Viewer {
    pub x: f64,
    pub z: f64,
    pub yaw: f32,
    pub view_distance: u32,
}

pub enum LoadEvent {
    Loaded(Column, BiomeMap),
    Unloaded(ColumnPos),
}

pub struct ColumnLoader {
    workers: GenerationWorkers,
    /// Columns changed by players: read before generating, written when unloaded.
    store: RegionStore,
    /// Load order for the last centre, heading sector and distance (recomputed on change).
    order: Option<((ColumnPos, i32, u32), Vec<ColumnPos>)>,
    columns: HashMap<ColumnPos, Column>,
    /// Never saved: recomputed from the seed for columns read from a save.
    biomes: HashMap<ColumnPos, BiomeMap>,
    requested: HashSet<ColumnPos>,
    biomes_requested: HashSet<ColumnPos>,
    /// Columns read from a save, waiting for their biomes.
    without_biomes: HashSet<ColumnPos>,
    sent: HashSet<ColumnPos>,
    modified: HashSet<ColumnPos>,
    /// Columns put in place since the last `take_arrived`.
    arrived: Vec<ColumnPos>,
}

impl ColumnLoader {
    pub fn new(workers: GenerationWorkers, store: RegionStore) -> Self {
        Self {
            workers,
            store,
            order: None,
            columns: HashMap::new(),
            biomes: HashMap::new(),
            requested: HashSet::new(),
            biomes_requested: HashSet::new(),
            without_biomes: HashSet::new(),
            sent: HashSet::new(),
            modified: HashSet::new(),
            arrived: Vec::new(),
        }
    }

    pub fn update(&mut self, viewer: &Viewer) -> Vec<LoadEvent> {
        self.receive_generated();
        let center = column_of(viewer);
        let mut events = self.unload_far(center, viewer.view_distance);
        let wanted = self.load_order(center, viewer);
        self.request_missing(&wanted);
        self.request_biomes();
        let ready: Vec<ColumnPos> = wanted
            .iter()
            .filter(|p| !self.sent.contains(p) && self.columns.contains_key(p))
            .filter(|p| self.biomes.contains_key(p))
            .take(SENT_PER_TICK)
            .copied()
            .collect();
        for pos in ready {
            if let (Some(column), Some(biomes)) = (self.columns.get(&pos), self.biomes.get(&pos)) {
                self.sent.insert(pos);
                events.push(LoadEvent::Loaded(column.clone(), biomes.clone()));
            }
        }
        events
    }

    fn receive_generated(&mut self) {
        while let Some(generated) = self.workers.finished().next() {
            self.accept(generated);
        }
    }

    /// Waits up to `timeout` for one generation result; `false` if none came.
    #[cfg(test)]
    pub fn wait_generated(&mut self, timeout: std::time::Duration) -> bool {
        let next = self.workers.wait_finished(timeout);
        let came = next.is_some();
        next.into_iter()
            .for_each(|generated| self.accept(generated));
        came
    }

    fn accept(&mut self, generated: Generated) {
        match generated {
            // A result for a place left and asked again may arrive twice: the first
            // one in may already have been changed by a player.
            Generated::Column(column, biomes) => {
                let pos = column.pos;
                if self.requested.remove(&pos) && !self.columns.contains_key(&pos) {
                    self.arrived.push(pos);
                    self.columns.insert(pos, column);
                    self.biomes.insert(pos, biomes);
                }
            }
            Generated::Biomes(pos, biomes) => {
                if self.biomes_requested.remove(&pos) && self.columns.contains_key(&pos) {
                    self.without_biomes.remove(&pos);
                    self.biomes.insert(pos, biomes);
                }
            }
        }
    }

    /// Columns read from a save get their biomes from the generation threads.
    fn request_biomes(&mut self) {
        let missing: Vec<ColumnPos> = self
            .without_biomes
            .iter()
            .filter(|p| !self.biomes_requested.contains(p))
            .copied()
            .collect();
        for pos in missing {
            if !self.workers.request_biomes(pos) {
                break;
            }
            self.biomes_requested.insert(pos);
        }
    }

    /// Writes every modified column (loaded or not) to disk.
    pub fn save(&mut self) -> Result<(), SaveError> {
        for pos in self.modified.drain() {
            if let Some(column) = self.columns.get(&pos) {
                self.store.store(column);
            }
        }
        self.store.flush()
    }

    /// Puts a ready-made column in place, as if just loaded.
    #[cfg(test)]
    pub fn insert(&mut self, column: Column) {
        self.arrived.push(column.pos);
        let biomes = BiomeMap::uniform(strata_core::chunk::BiomeId(0));
        self.biomes.insert(column.pos, biomes);
        self.columns.insert(column.pos, column);
    }

    /// The columns loaded (generated or read) since the last call.
    pub fn take_arrived(&mut self) -> Vec<ColumnPos> {
        std::mem::take(&mut self.arrived)
    }

    pub fn column(&self, pos: ColumnPos) -> Option<&Column> {
        self.columns.get(&pos)
    }

    /// `None` when the column is not loaded.
    pub fn block(&self, pos: BlockPos) -> Option<BlockId> {
        let column = self.columns.get(&pos.column())?;
        let (x, z) = (
            pos.x.rem_euclid(SECTION_SIZE),
            pos.z.rem_euclid(SECTION_SIZE),
        );
        Some(column.get(x as u32, pos.y, z as u32))
    }

    /// Changes a block in a loaded column; `false` if the column is not loaded.
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        let Some(column) = self.columns.get_mut(&pos.column()) else {
            return false;
        };
        let (x, z) = (
            pos.x.rem_euclid(SECTION_SIZE),
            pos.z.rem_euclid(SECTION_SIZE),
        );
        column.set(x as u32, pos.y, z as u32, block);
        self.modified.insert(pos.column());
        true
    }

    fn load_order(&mut self, center: ColumnPos, viewer: &Viewer) -> Vec<ColumnPos> {
        let sector = (viewer.yaw / std::f32::consts::FRAC_PI_4).round() as i32;
        let key = (center, sector.rem_euclid(8), viewer.view_distance);
        match &self.order {
            Some((cached, order)) if *cached == key => order.clone(),
            _ => {
                let order = wanted(center, viewer);
                self.order = Some((key, order.clone()));
                order
            }
        }
    }

    fn request_missing(&mut self, wanted: &[ColumnPos]) {
        let room = self.workers.capacity().saturating_sub(self.requested.len());
        let saved: Vec<ColumnPos> = wanted
            .iter()
            .filter(|p| !self.columns.contains_key(p) && self.store.has(**p))
            .take(SAVED_PER_TICK)
            .copied()
            .collect();
        for pos in saved {
            if let Some(column) = self.store.load(pos) {
                self.arrived.push(pos);
                self.columns.insert(pos, column);
                self.without_biomes.insert(pos);
            }
        }
        // Saved columns wait for their turn to be read; they are never generated.
        let missing: Vec<ColumnPos> = wanted
            .iter()
            .filter(|p| !self.columns.contains_key(p) && !self.requested.contains(p))
            .filter(|p| !self.store.has(**p))
            .take(room)
            .copied()
            .collect();
        for pos in missing {
            if !self.workers.request(pos) {
                break;
            }
            self.requested.insert(pos);
        }
    }

    fn unload_far(&mut self, center: ColumnPos, view_distance: u32) -> Vec<LoadEvent> {
        let limit = view_distance as i32 + UNLOAD_MARGIN;
        let far = |p: &ColumnPos| (p.x - center.x).abs().max((p.z - center.z).abs()) > limit;
        let gone: Vec<ColumnPos> = self.columns.keys().copied().filter(far).collect();
        self.requested.retain(|p| !far(p));
        self.biomes_requested.retain(|p| !far(p));
        self.without_biomes.retain(|p| !far(p));
        gone.into_iter()
            .filter_map(|pos| {
                self.biomes.remove(&pos);
                if let Some(column) = self.columns.remove(&pos)
                    && self.modified.remove(&pos)
                {
                    self.store.store(&column);
                }
                self.sent.remove(&pos).then_some(LoadEvent::Unloaded(pos))
            })
            .collect()
    }
}

fn column_of(viewer: &Viewer) -> ColumnPos {
    let size = f64::from(SECTION_SIZE);
    ColumnPos {
        x: (viewer.x / size).floor() as i32,
        z: (viewer.z / size).floor() as i32,
    }
}

/// Columns within a circle of `view_distance`, nearest first, those in front of the
/// player before those behind at equal distance.
fn wanted(center: ColumnPos, viewer: &Viewer) -> Vec<ColumnPos> {
    let r = viewer.view_distance as i32;
    let (sin, cos) = viewer.yaw.sin_cos();
    let forward = (sin, -cos);
    let mut columns: Vec<(i64, ColumnPos)> = (-r..=r)
        .flat_map(|dz| (-r..=r).map(move |dx| (dx, dz)))
        .filter(|&(dx, dz)| strata_protocol::in_view_disc(dx, dz, viewer.view_distance))
        .map(|(dx, dz)| {
            let distance = (dx * dx + dz * dz) as f32;
            let facing = (dx as f32 * forward.0 + dz as f32 * forward.1) / distance.sqrt().max(1.0);
            let priority = (distance * (1.25 - 0.5 * facing) * 16.0) as i64;
            (
                priority,
                ColumnPos {
                    x: center.x + dx,
                    z: center.z + dz,
                },
            )
        })
        .collect();
    columns.sort_by_key(|(priority, pos)| (*priority, pos.x, pos.z));
    columns.into_iter().map(|(_, pos)| pos).collect()
}

/// The loaded columns inside the border, as the block updates see them.
pub struct Bounded<'a> {
    pub loader: &'a mut ColumnLoader,
    pub border: i32,
}

impl BlockAccess for Bounded<'_> {
    fn block(&self, pos: BlockPos) -> Option<BlockId> {
        self.loader.block(pos)
    }

    fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        pos.is_valid(self.border) && self.loader.set_block(pos, block)
    }

    fn contains(&self, pos: BlockPos) -> bool {
        pos.is_valid(self.border)
    }
}

#[cfg(test)]
pub mod tests;
