use crossbeam_channel::{Receiver, Sender, TrySendError, bounded, unbounded};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use strata_core::chunk::{BiomeMap, Column};
use strata_core::coords::ColumnPos;
use strata_worldgen::Generator;

#[derive(Debug, Clone, Copy)]
enum Job {
    Column(ColumnPos),
    /// Only the biomes, for a column read from a save.
    Biomes(ColumnPos),
}

pub enum Generated {
    Column(Column, BiomeMap),
    Biomes(ColumnPos, BiomeMap),
}

impl Job {
    fn run(self, generator: &Generator) -> Generated {
        match self {
            Self::Column(pos) => {
                let (column, biomes) = generator.generate_with_biomes(pos);
                Generated::Column(column, biomes)
            }
            Self::Biomes(pos) => Generated::Biomes(pos, generator.biome_map(pos)),
        }
    }
}

/// Background threads generating columns. Bounded queue: the server asks again later
/// rather than piling up requests for places the player has already left.
pub struct GenerationWorkers {
    jobs: Option<Sender<Job>>,
    results: Receiver<Generated>,
    #[cfg(test)]
    generator: Arc<Generator>,
    threads: Vec<JoinHandle<()>>,
    capacity: usize,
}

impl GenerationWorkers {
    pub fn spawn(count: usize, generator: Arc<Generator>) -> std::io::Result<Self> {
        let capacity = count.max(1) * 4;
        let (jobs, job_receiver) = bounded::<Job>(capacity);
        let (result_sender, results) = unbounded();
        let threads = (0..count.max(1))
            .map(|index| {
                let jobs = job_receiver.clone();
                let results = result_sender.clone();
                let generator = Arc::clone(&generator);
                thread::Builder::new()
                    .name(format!("worldgen-{index}"))
                    .spawn(move || {
                        for job in jobs {
                            if results.send(job.run(&generator)).is_err() {
                                return;
                            }
                        }
                    })
            })
            .collect::<std::io::Result<_>>()?;
        Ok(Self {
            #[cfg(test)]
            generator,
            jobs: Some(jobs),
            results,
            threads,
            capacity,
        })
    }

    #[cfg(test)]
    pub fn generator(&self) -> &Generator {
        &self.generator
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// `false` when the queue is full.
    pub fn request(&self, pos: ColumnPos) -> bool {
        self.send(Job::Column(pos))
    }

    /// `false` when the queue is full.
    pub fn request_biomes(&self, pos: ColumnPos) -> bool {
        self.send(Job::Biomes(pos))
    }

    fn send(&self, job: Job) -> bool {
        self.jobs.as_ref().is_some_and(|jobs| {
            !matches!(
                jobs.try_send(job),
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_))
            )
        })
    }

    pub fn finished(&self) -> impl Iterator<Item = Generated> + '_ {
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
