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

/// One batch of jobs done: the columns lit for the first time, and the sections whose
/// light changed (or the light of their border), with their new light.
#[derive(Default)]
pub struct LightResult {
    pub lit: Vec<ColumnPos>,
    pub sections: Vec<(ColumnPos, usize, SectionLight)>,
    pub(crate) jobs: usize,
}

pub struct LightWorker {
    jobs: Option<Sender<LightJob>>,
    results: Receiver<LightResult>,
    thread: Option<JoinHandle<()>>,
    /// Jobs sent whose result has not been taken yet.
    outstanding: usize,
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
            outstanding: 0,
        })
    }

    pub fn insert(&mut self, column: Arc<Column>) {
        self.send(LightJob::Insert(column));
    }

    pub fn remove(&mut self, pos: ColumnPos) {
        self.send(LightJob::Remove(pos));
    }

    pub fn changed(&mut self, column: Arc<Column>, changes: Vec<(BlockPos, BlockId)>) {
        self.send(LightJob::Changed(column, changes));
    }

    pub fn finished(&mut self) -> Vec<LightResult> {
        let results: Vec<_> = self.results.try_iter().collect();
        let done: usize = results.iter().map(|result| result.jobs).sum();
        self.outstanding = self.outstanding.saturating_sub(done);
        results
    }

    /// Every job sent so far is done and its result taken.
    pub fn is_idle(&self) -> bool {
        self.outstanding == 0
    }

    fn send(&mut self, job: LightJob) {
        let Some(jobs) = &self.jobs else {
            return;
        };
        if jobs.send(job).is_ok() {
            self.outstanding += 1;
        } else {
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
        let mut result = LightResult {
            jobs: 1,
            ..LightResult::default()
        };
        state.apply(first, table, &mut result.lit);
        for job in jobs.try_iter() {
            state.apply(job, table, &mut result.lit);
            result.jobs += 1;
        }
        result.sections = state.changed_sections();
        if results.send(result).is_err() {
            return;
        }
    }
}

impl LightState {
    fn apply(&mut self, job: LightJob, table: &LightTable, lit: &mut Vec<ColumnPos>) {
        match job {
            LightJob::Insert(column) => {
                let pos = column.pos;
                self.blocks.insert(pos, column);
                for near in around(pos) {
                    if !self.lighting.is_lit(near)
                        && self.is_complete(near)
                        && self.lighting.light_column(&self.blocks, table, near)
                    {
                        lit.push(near);
                    }
                }
            }
            LightJob::Remove(pos) => {
                self.blocks.remove(&pos);
                self.lighting.remove_column(pos);
                lit.retain(|&p| p != pos);
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

    fn changed_sections(&mut self) -> Vec<(ColumnPos, usize, SectionLight)> {
        self.lighting
            .take_changed()
            .into_iter()
            .filter_map(|(pos, index)| {
                let light = self.lighting.column(pos)?.section(index)?.clone();
                Some((pos, index, light))
            })
            .collect()
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

    fn wait(worker: &mut LightWorker) -> (Vec<ColumnPos>, Vec<(ColumnPos, usize, SectionLight)>) {
        let start = Instant::now();
        let (mut lit, mut sections) = (Vec::new(), Vec::new());
        while start.elapsed() < Duration::from_secs(10) {
            for result in worker.finished() {
                lit.extend(result.lit);
                sections.extend(result.sections);
            }
            if worker.is_idle() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        (lit, sections)
    }

    #[test]
    fn columns_are_lit_once_their_neighbours_arrive_and_torches_update_them() {
        let mut worker = LightWorker::spawn(table()).unwrap();
        let mut columns = std::collections::HashMap::new();
        for pos in around(ColumnPos { x: 0, z: 0 }) {
            let column = Arc::new(Column::empty(pos));
            columns.insert(pos, Arc::clone(&column));
            worker.insert(column);
        }
        let origin = ColumnPos { x: 0, z: 0 };
        let (lit, sections) = wait(&mut worker);
        assert_eq!(lit, vec![origin], "only the column with its 8 neighbours");
        assert_eq!(sections.len(), 32);
        assert!(sections.iter().all(|(pos, _, _)| *pos == origin));
        let mut column = Arc::clone(&columns[&origin]);
        Arc::make_mut(&mut column).set(4, 100, 4, BlockId(1));
        worker.changed(column, vec![(BlockPos::new(4, 100, 4), BlockId::AIR)]);
        let (lit, changed) = wait(&mut worker);
        assert!(lit.is_empty());
        let section = changed
            .iter()
            .find(|(pos, index, _)| *pos == origin && *index == 6)
            .map(|(_, _, light)| light.clone());
        let cell = strata_core::light::section_index(4, 100 - 96, 4);
        assert_eq!(section.map(|s| s.get(cell)), Some(PackedLight::new(15, 14)));
    }
}
