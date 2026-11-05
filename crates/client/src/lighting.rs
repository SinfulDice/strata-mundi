//! Light computed on its own background thread from a copy of the blocks the client holds
//! (shared, not duplicated: columns are reference-counted and copied only on change).
//! Light is derived data, so the client may compute it without ever writing the world.

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnMap, ColumnPos};
use strata_core::light::{LightTable, Lighting, SectionLight};

/// A batch ends after this many jobs or this long, so results flow while columns stream in.
const BATCH_JOBS: usize = 16;
const BATCH_TIME: Duration = Duration::from_millis(4);

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
    stop: Arc<AtomicBool>,
    /// Jobs sent whose result has not been taken yet.
    outstanding: usize,
    failed: bool,
}

impl LightWorker {
    pub fn spawn(table: LightTable) -> std::io::Result<Self> {
        let (jobs, job_receiver) = unbounded();
        let (result_sender, results) = unbounded();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("lighting".into())
            .spawn(move || work(&job_receiver, &result_sender, &table, &stopping))?;
        Ok(Self {
            jobs: Some(jobs),
            results,
            thread: Some(thread),
            stop,
            outstanding: 0,
            failed: false,
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
        let mut results = Vec::new();
        loop {
            match self.results.try_recv() {
                Ok(result) => results.push(result),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.fail();
                    break;
                }
            }
        }
        let done: usize = results.iter().map(|result| result.jobs).sum();
        self.outstanding = self.outstanding.saturating_sub(done);
        results
    }

    /// Every job sent so far is done and its result taken (or the thread is gone).
    pub fn is_idle(&self) -> bool {
        self.outstanding == 0 || self.failed
    }

    /// The thread stopped on its own (a bug): reported once, the world is drawn without light.
    pub fn has_failed(&self) -> bool {
        self.failed
    }

    fn fail(&mut self) {
        if !self.failed {
            self.failed = true;
            tracing::error!("the lighting thread stopped: the world is drawn without light");
        }
    }

    fn send(&mut self, job: LightJob) {
        let sent = self
            .jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(job).is_ok());
        if sent {
            self.outstanding += 1;
        } else {
            self.fail();
        }
    }
}

impl Drop for LightWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Jobs received and not done yet: block changes and removals first (what the player
/// does shows at once), then columns to light, in arrival order.
#[derive(Default)]
struct Pending {
    edits: VecDeque<LightJob>,
    inserts: VecDeque<Arc<Column>>,
    /// Jobs merged into a later one, reported done with the next batch.
    merged: usize,
}

impl Pending {
    fn is_empty(&self) -> bool {
        self.edits.is_empty() && self.inserts.is_empty()
    }

    /// Keeps each column's jobs in order although edits jump ahead of inserts.
    fn push(&mut self, job: LightJob) {
        match job {
            LightJob::Insert(column) => {
                self.drop_inserts(column.pos);
                self.inserts.push_back(column);
            }
            LightJob::Changed(column, changes) => {
                match self.inserts.iter_mut().find(|c| c.pos == column.pos) {
                    // Lit from scratch with the new blocks anyway.
                    Some(waiting) => {
                        *waiting = column;
                        self.merged += 1;
                    }
                    None => self.edits.push_back(LightJob::Changed(column, changes)),
                }
            }
            LightJob::Remove(pos) => {
                self.drop_inserts(pos);
                self.edits.push_back(LightJob::Remove(pos));
            }
        }
    }

    fn drop_inserts(&mut self, pos: ColumnPos) {
        let before = self.inserts.len();
        self.inserts.retain(|column| column.pos != pos);
        self.merged += before - self.inserts.len();
    }

    fn pop(&mut self) -> Option<LightJob> {
        self.edits
            .pop_front()
            .or_else(|| self.inserts.pop_front().map(LightJob::Insert))
    }
}

fn work(
    jobs: &Receiver<LightJob>,
    results: &Sender<LightResult>,
    table: &LightTable,
    stop: &AtomicBool,
) {
    let mut state = LightState::default();
    let mut pending = Pending::default();
    while !stop.load(Ordering::Relaxed) {
        if pending.is_empty() {
            match jobs.recv() {
                Ok(job) => pending.push(job),
                Err(_) => return,
            }
        }
        jobs.try_iter().for_each(|job| pending.push(job));
        let Some(result) = state.batch(&mut pending, table, stop) else {
            return;
        };
        if results.send(result).is_err() {
            return;
        }
    }
}

/// The blocks and light of the thread.
#[derive(Default)]
struct LightState {
    blocks: ColumnMap<Arc<Column>>,
    lighting: Lighting,
}

impl LightState {
    /// `None` when asked to stop.
    fn batch(
        &mut self,
        pending: &mut Pending,
        table: &LightTable,
        stop: &AtomicBool,
    ) -> Option<LightResult> {
        let _span = tracing::trace_span!("lighting").entered();
        let start = Instant::now();
        let mut result = LightResult {
            jobs: std::mem::take(&mut pending.merged),
            ..LightResult::default()
        };
        let mut done = 0;
        while done < BATCH_JOBS && start.elapsed() < BATCH_TIME {
            let Some(job) = pending.pop() else {
                break;
            };
            if stop.load(Ordering::Relaxed) {
                return None;
            }
            self.apply(job, table, &mut result.lit);
            done += 1;
        }
        result.jobs += done;
        result.sections = self.changed_sections();
        Some(result)
    }

    fn apply(&mut self, job: LightJob, table: &LightTable, lit: &mut Vec<ColumnPos>) {
        match job {
            LightJob::Insert(column) => {
                let pos = column.pos;
                self.blocks.insert(pos, column);
                // New blocks for a lit column (sent again by the server): lit again.
                self.lighting.remove_column(pos);
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
mod tests;
