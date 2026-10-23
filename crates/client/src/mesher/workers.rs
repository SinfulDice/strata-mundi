use super::{BlockTable, Neighborhood, SectionMesh, mesh_section};
use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Which section a mesh belongs to: column x/z and section index.
pub type SectionKey = (i32, i32, usize);

pub struct MeshJob {
    pub key: SectionKey,
    /// Bumped each time the section changes, so outdated results can be dropped.
    pub version: u64,
    pub blocks: Neighborhood,
}

pub struct MeshResult {
    pub key: SectionKey,
    pub version: u64,
    pub mesh: SectionMesh,
}

/// Background threads that mesh sections. The queue is bounded: when it is full, the
/// caller keeps the job for a later frame instead of piling up work.
pub struct MeshWorkers {
    jobs: Option<Sender<MeshJob>>,
    results: Receiver<MeshResult>,
    threads: Vec<JoinHandle<()>>,
}

impl MeshWorkers {
    pub fn spawn(
        count: usize,
        table: Arc<BlockTable>,
        queue: usize,
        ambient_occlusion: bool,
    ) -> std::io::Result<Self> {
        let (jobs, job_receiver) = bounded::<MeshJob>(queue.max(1));
        let (result_sender, results) = unbounded();
        let threads = (0..count.max(1))
            .map(|index| {
                let jobs = job_receiver.clone();
                let results = result_sender.clone();
                let table = Arc::clone(&table);
                thread::Builder::new()
                    .name(format!("mesher-{index}"))
                    .spawn(move || work(&jobs, &results, &table, ambient_occlusion))
            })
            .collect::<std::io::Result<_>>()?;
        Ok(Self {
            jobs: Some(jobs),
            results,
            threads,
        })
    }

    /// Gives the job back when the queue is full.
    pub fn submit(&self, job: MeshJob) -> Result<(), MeshJob> {
        let Some(jobs) = &self.jobs else {
            return Err(job);
        };
        jobs.try_send(job).map_err(|error| match error {
            TrySendError::Full(job) | TrySendError::Disconnected(job) => job,
        })
    }

    pub fn finished(&self) -> impl Iterator<Item = MeshResult> + '_ {
        self.results.try_iter()
    }
}

impl Drop for MeshWorkers {
    fn drop(&mut self) {
        self.jobs = None;
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

fn work(
    jobs: &Receiver<MeshJob>,
    results: &Sender<MeshResult>,
    table: &BlockTable,
    ambient_occlusion: bool,
) {
    for job in jobs {
        let _span = tracing::trace_span!("mesh_section").entered();
        let mesh = mesh_section(&job.blocks, table, ambient_occlusion);
        let result = MeshResult {
            key: job.key,
            version: job.version,
            mesh,
        };
        if results.send(result).is_err() {
            return;
        }
    }
}
