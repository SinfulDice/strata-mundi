use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use strata_core::chunk::Column;
use strata_core::coords::ColumnPos;
use strata_worldgen::Generator;

/// Background threads generating columns. Bounded queue: the server asks again later
/// rather than piling up requests for places the player has already left.
pub struct GenerationWorkers {
    jobs: Option<Sender<ColumnPos>>,
    results: Receiver<Column>,
    threads: Vec<JoinHandle<()>>,
    capacity: usize,
}

impl GenerationWorkers {
    pub fn spawn(count: usize, generator: Arc<Generator>) -> std::io::Result<Self> {
        let capacity = count.max(1) * 4;
        let (jobs, job_receiver) = bounded::<ColumnPos>(capacity);
        let (result_sender, results) = unbounded();
        let threads = (0..count.max(1))
            .map(|index| {
                let jobs = job_receiver.clone();
                let results = result_sender.clone();
                let generator = Arc::clone(&generator);
                thread::Builder::new()
                    .name(format!("worldgen-{index}"))
                    .spawn(move || {
                        for pos in jobs {
                            if results.send(generator.generate(pos)).is_err() {
                                return;
                            }
                        }
                    })
            })
            .collect::<std::io::Result<_>>()?;
        Ok(Self {
            jobs: Some(jobs),
            results,
            threads,
            capacity,
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// `false` when the queue is full.
    pub fn request(&self, pos: ColumnPos) -> bool {
        self.jobs.as_ref().is_some_and(|jobs| {
            !matches!(
                jobs.try_send(pos),
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_))
            )
        })
    }

    pub fn finished(&self) -> impl Iterator<Item = Column> + '_ {
        self.results.try_iter()
    }
}

impl Drop for GenerationWorkers {
    fn drop(&mut self) {
        self.jobs = None;
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
