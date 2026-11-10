//! Buffers of every sound, made the first time each variant is asked and kept: rendered
//! from the recipe, or decoded from the pack's `.ogg`. The work is done on a thread of
//! its own; a sound not ready yet is not played (it is the next time).

use super::decode::decode;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;
use strata_core::audio::{self, Rng, Sound, Sounds};
use strata_core::content::{RegistryId, SoundId};

/// One play of a sound: which buffer and how fast.
#[derive(Debug, Clone)]
pub struct Pick {
    pub buffer: Arc<[f32]>,
    pub pitch: f32,
}

#[derive(Debug, Clone, Default)]
enum Slot {
    #[default]
    Missing,
    Asked,
    Ready(Arc<[f32]>),
    /// The recording could not be decoded: the recipe is used.
    Failed,
}

#[derive(Default)]
struct Entry {
    variants: Vec<Slot>,
    recording: Slot,
    last_variant: Option<usize>,
}

/// Which buffer of a sound: a variant of the recipe, or the recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Variant(usize),
    Recording,
}

struct Job {
    id: SoundId,
    part: Part,
    sound: Sound,
}

type Made = (SoundId, Part, Option<Arc<[f32]>>);

#[derive(Default)]
pub struct SoundBank {
    sounds: Sounds,
    entries: Vec<Entry>,
    worker: Option<(Sender<Job>, Receiver<Made>)>,
}

impl SoundBank {
    pub fn new(sounds: Sounds) -> Self {
        let entries = sounds
            .registry
            .iter()
            .map(|(_, _, sound)| Entry {
                variants: vec![Slot::Missing; usize::from(sound.def.variants)],
                ..Entry::default()
            })
            .collect();
        Self {
            sounds,
            entries,
            worker: None,
        }
    }

    pub fn sound(&self, id: SoundId) -> Option<&Sound> {
        self.sounds.get(id)
    }

    /// A buffer and a pitch for one play, never the same variant twice in a row; `None`
    /// while the buffer is being made.
    pub fn pick(&mut self, id: SoundId, rng: &mut Rng) -> Option<Pick> {
        let sound = self.sounds.get(id)?;
        let pitch = rng.range(sound.def.pitch);
        let entry = self.entries.get(id.index())?;
        let count = entry.variants.len();
        let mut index = (rng.next_u64() % count.max(1) as u64) as usize;
        if count > 1 && entry.last_variant == Some(index) {
            index = (index + 1) % count;
        }
        let buffer = self.buffer(id, index)?;
        if let Some(entry) = self.entries.get_mut(id.index()) {
            entry.last_variant = Some(index);
        }
        Some(Pick { buffer, pitch })
    }

    /// The buffer of a looping bed (its first variant, or its recording).
    pub fn bed(&mut self, id: SoundId) -> Option<Arc<[f32]>> {
        self.buffer(id, 0)
    }

    /// Keeps the buffers made since the last call.
    pub fn collect(&mut self) {
        let Some(made) = self.worker.as_ref().map(|(_, made)| made.clone()) else {
            return;
        };
        for (id, part, buffer) in made.try_iter() {
            self.store(id, part, buffer);
        }
    }

    /// The pack's recording if it has a readable one, else variant `index` of the recipe.
    fn buffer(&mut self, id: SoundId, index: usize) -> Option<Arc<[f32]>> {
        let sound = self.sounds.get(id)?;
        let entry = self.entries.get(id.index())?;
        let (part, slot) = match (&sound.ogg, &entry.recording) {
            (Some(_), slot) if !matches!(slot, Slot::Failed) => (Part::Recording, slot),
            _ => (Part::Variant(index), entry.variants.get(index)?),
        };
        match slot {
            Slot::Ready(buffer) => Some(Arc::clone(buffer)),
            Slot::Asked | Slot::Failed => None,
            Slot::Missing => {
                self.ask(id, part);
                None
            }
        }
    }

    fn ask(&mut self, id: SoundId, part: Part) {
        let Some(sound) = self.sounds.get(id).cloned() else {
            return;
        };
        if self.worker.is_none() {
            self.worker = spawn_worker();
        }
        let job = Job { id, part, sound };
        let job = match &self.worker {
            Some((jobs, _)) => match jobs.send(job) {
                Ok(()) => {
                    self.set(id, part, Slot::Asked);
                    return;
                }
                Err(lost) => lost.into_inner(),
            },
            None => job,
        };
        let (id, part, buffer) = make(job);
        self.store(id, part, buffer);
    }

    fn store(&mut self, id: SoundId, part: Part, buffer: Option<Arc<[f32]>>) {
        self.set(id, part, buffer.map_or(Slot::Failed, Slot::Ready));
    }

    fn set(&mut self, id: SoundId, part: Part, value: Slot) {
        let Some(entry) = self.entries.get_mut(id.index()) else {
            return;
        };
        let slot = match part {
            Part::Recording => Some(&mut entry.recording),
            Part::Variant(index) => entry.variants.get_mut(index),
        };
        if let Some(slot) = slot {
            *slot = value;
        }
    }

    /// Waits for every buffer asked.
    #[cfg(test)]
    pub fn wait_ready(&mut self) {
        let asked = |bank: &Self| {
            bank.entries.iter().any(|entry| {
                entry
                    .variants
                    .iter()
                    .chain(std::iter::once(&entry.recording))
                    .any(|slot| matches!(slot, Slot::Asked))
            })
        };
        while asked(self) {
            self.collect();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// Samples kept in memory.
    #[cfg(test)]
    pub fn cached_samples(&self) -> usize {
        self.entries
            .iter()
            .flat_map(|entry| {
                entry
                    .variants
                    .iter()
                    .chain(std::iter::once(&entry.recording))
            })
            .map(|slot| match slot {
                Slot::Ready(buffer) => buffer.len(),
                _ => 0,
            })
            .sum()
    }
}

fn make(job: Job) -> Made {
    let buffer = match job.part {
        Part::Recording => job.sound.ogg.as_deref().and_then(decode),
        Part::Variant(index) => {
            let seed = audio::variant_seed(&job.sound, index as u8);
            Some(audio::render(&job.sound.def, seed).into())
        }
    };
    (job.id, job.part, buffer)
}

/// The thread that makes the buffers; `None` (and the work done in place) without one.
fn spawn_worker() -> Option<(Sender<Job>, Receiver<Made>)> {
    let (jobs, job_receiver) = crossbeam_channel::unbounded::<Job>();
    let (made_sender, made) = crossbeam_channel::unbounded();
    let spawned = std::thread::Builder::new()
        .name("sound-bank".to_owned())
        .spawn(move || {
            for job in job_receiver {
                if made_sender.send(make(job)).is_err() {
                    break;
                }
            }
        });
    match spawned {
        Ok(_) => Some((jobs, made)),
        Err(error) => {
            tracing::warn!(%error, "no sound thread: sounds are made on first play");
            None
        }
    }
}
