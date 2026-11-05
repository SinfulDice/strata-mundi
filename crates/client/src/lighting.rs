//! Light computed on its own background thread from a copy of the blocks the client holds
//! (shared, not duplicated: columns are reference-counted and copied only on change).
//! Light is derived data, so the client may compute it without ever writing the world.

use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnMap, ColumnPos};
use strata_core::light::{LightTable, Lighting, SectionLight};

enum LightJob {
    Insert(Arc<Column>),
    Remove(ColumnPos),
    /// The column's new blocks, and each changed position with its previous block.
    Changed(Arc<Column>, Vec<(BlockPos, BlockId)>),
}

/// Sections whose light changed (or the light of their border), with their new light.
pub struct LightResult {
    pub sections: Vec<(ColumnPos, usize, SectionLight)>,
}

pub struct LightWorker {
    jobs: Option<Sender<LightJob>>,
    results: Receiver<LightResult>,
    thread: Option<JoinHandle<()>>,
}

impl LightWorker {
    pub fn spawn(table: LightTable) -> std::io::Result<Self> {
        let (jobs, job_receiver) = unbounded();
        let (result_sender, results) = unbounded();
        let thread = thread::Builder::new()
            .name("lighting".into())
            .spawn(move || work(&job_receiver, &result_sender, &table))?;
        Ok(Self {
            jobs: Some(jobs),
            results,
            thread: Some(thread),
        })
    }

    pub fn insert(&self, column: Arc<Column>) {
        self.send(LightJob::Insert(column));
    }

    pub fn remove(&self, pos: ColumnPos) {
        self.send(LightJob::Remove(pos));
    }

    pub fn changed(&self, column: Arc<Column>, changes: Vec<(BlockPos, BlockId)>) {
        self.send(LightJob::Changed(column, changes));
    }

    pub fn finished(&self) -> impl Iterator<Item = LightResult> + '_ {
        self.results.try_iter()
    }

    fn send(&self, job: LightJob) {
        if let Some(jobs) = &self.jobs
            && jobs.send(job).is_err()
        {
            tracing::warn!("the lighting thread stopped");
        }
    }
}

impl Drop for LightWorker {
    fn drop(&mut self) {
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The blocks and light of the thread.
#[derive(Default)]
struct LightState {
    blocks: ColumnMap<Arc<Column>>,
    lighting: Lighting,
}

fn work(jobs: &Receiver<LightJob>, results: &Sender<LightResult>, table: &LightTable) {
    let mut state = LightState::default();
    while let Ok(first) = jobs.recv() {
        let _span = tracing::trace_span!("lighting").entered();
        state.apply(first, table);
        for job in jobs.try_iter() {
            state.apply(job, table);
        }
        let result = state.changed_sections();
        if !result.sections.is_empty() && results.send(result).is_err() {
            return;
        }
    }
}

impl LightState {
    fn apply(&mut self, job: LightJob, table: &LightTable) {
        match job {
            LightJob::Insert(column) => {
                let pos = column.pos;
                self.blocks.insert(pos, column);
                for near in around(pos) {
                    if !self.lighting.is_lit(near) && self.is_complete(near) {
                        self.lighting.light_column(&self.blocks, table, near);
                    }
                }
            }
            LightJob::Remove(pos) => {
                self.blocks.remove(&pos);
                self.lighting.remove_column(pos);
            }
            LightJob::Changed(column, changes) => {
                self.blocks.insert(column.pos, column);
                for (pos, old) in changes {
                    self.lighting.block_changed(&self.blocks, table, pos, old);
                }
            }
        }
    }

    /// Lit once its eight neighbours are loaded, like meshing.
    fn is_complete(&self, pos: ColumnPos) -> bool {
        around(pos).all(|near| self.blocks.contains_key(&near))
    }

    fn changed_sections(&mut self) -> LightResult {
        let sections = self
            .lighting
            .take_changed()
            .into_iter()
            .filter_map(|(pos, index)| {
                let light = self.lighting.column(pos)?.section(index)?.clone();
                Some((pos, index, light))
            })
            .collect();
        LightResult { sections }
    }
}

/// The column and its eight neighbours.
fn around(pos: ColumnPos) -> impl Iterator<Item = ColumnPos> {
    (-1..=1).flat_map(move |dz| {
        (-1..=1).map(move |dx| ColumnPos {
            x: pos.x + dx,
            z: pos.z + dz,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use strata_core::content::Block;
    use strata_core::light::PackedLight;

    fn table() -> LightTable {
        let torch = Block {
            light_emission: 14,
            ..Block::air()
        };
        LightTable::new([Block::air(), torch].iter())
    }

    fn wait(worker: &LightWorker) -> Vec<(ColumnPos, usize, SectionLight)> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            let sections: Vec<_> = worker.finished().flat_map(|r| r.sections).collect();
            if !sections.is_empty() {
                return sections;
            }
            thread::sleep(Duration::from_millis(5));
        }
        Vec::new()
    }

    #[test]
    fn columns_are_lit_once_their_neighbours_arrive_and_torches_update_them() {
        let worker = LightWorker::spawn(table()).unwrap();
        let mut columns = std::collections::HashMap::new();
        for pos in around(ColumnPos { x: 0, z: 0 }) {
            let column = Arc::new(Column::empty(pos));
            columns.insert(pos, Arc::clone(&column));
            worker.insert(column);
        }
        let lit = wait(&worker);
        assert!(
            lit.iter()
                .any(|(pos, _, _)| *pos == ColumnPos { x: 0, z: 0 })
        );
        assert!(
            lit.iter()
                .all(|(pos, _, _)| *pos == ColumnPos { x: 0, z: 0 })
        );
        let origin = ColumnPos { x: 0, z: 0 };
        let mut column = Arc::clone(&columns[&origin]);
        Arc::make_mut(&mut column).set(4, 100, 4, BlockId(1));
        worker.changed(column, vec![(BlockPos::new(4, 100, 4), BlockId::AIR)]);
        let changed = wait(&worker);
        let section = changed
            .iter()
            .find(|(pos, index, _)| *pos == origin && *index == 6)
            .map(|(_, _, light)| light.clone());
        let cell = strata_core::light::section_index(4, 100 - 96, 4);
        assert_eq!(section.map(|s| s.get(cell)), Some(PackedLight::new(15, 14)));
    }
}
