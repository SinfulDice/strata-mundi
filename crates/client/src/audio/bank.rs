//! Buffers of every sound, rendered from the recipe or decoded from the pack's `.ogg`, and
//! kept. With a device they are all made at loading, shortest first, on a thread of its
//! own (the game does not wait); else each one the first time it is asked.

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
    /// On its way; `hurried` once a play is waiting for it.
    Asked {
        hurried: bool,
    },
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

/// The sound thread: buffers asked by a play go before those made in advance.
struct Worker {
    urgent: Sender<Job>,
    background: Sender<Job>,
    made: Receiver<Made>,
}

#[derive(Default)]
pub struct SoundBank {
    sounds: Sounds,
    entries: Vec<Entry>,
    worker: Option<Worker>,
    /// Every buffer is being made, not only those asked; since when, until all are.
    preparing: bool,
    preparing_since: Option<std::time::Instant>,
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
            preparing: false,
            preparing_since: None,
        }
    }

    /// Asks every buffer now, the shortest sounds first (steps and breaks are ready
    /// within a few milliseconds, the long ambience beds last).
    pub fn prepare_all(&mut self) {
        self.preparing = true;
        self.preparing_since = Some(std::time::Instant::now());
        let mut ids: Vec<SoundId> = self.sounds.registry.iter().map(|(id, _, _)| id).collect();
        ids.sort_by(|a, b| self.length(*a).total_cmp(&self.length(*b)));
        for id in ids {
            match self.sounds.get(id).map(|sound| sound.ogg.is_some()) {
                Some(true) => self.ask_missing(id, Part::Recording),
                Some(false) => self.ask_variants(id),
                None => {}
            }
        }
    }

    fn length(&self, id: SoundId) -> f32 {
        self.sounds.get(id).map_or(0.0, |sound| sound.def.duration)
    }

    fn ask_variants(&mut self, id: SoundId) {
        let count = self.entries.get(id.index()).map_or(0, |e| e.variants.len());
        for index in 0..count {
            self.ask_missing(id, Part::Variant(index));
        }
    }

    fn ask_missing(&mut self, id: SoundId, part: Part) {
        if matches!(self.slot(id, part), Some(Slot::Missing)) {
            self.ask(id, part, false);
        }
    }

    fn slot(&self, id: SoundId, part: Part) -> Option<&Slot> {
        let entry = self.entries.get(id.index())?;
        match part {
            Part::Recording => Some(&entry.recording),
            Part::Variant(index) => entry.variants.get(index),
        }
    }

    /// A buffer of the sound is on its way (asked and not made yet).
    pub fn is_coming(&self, id: SoundId) -> bool {
        self.entries.get(id.index()).is_some_and(|entry| {
            entry
                .variants
                .iter()
                .chain(std::iter::once(&entry.recording))
                .any(|slot| matches!(slot, Slot::Asked { .. }))
        })
    }

    /// Every buffer of every sound is made (or its recording failed and the recipe is).
    pub fn is_complete(&self) -> bool {
        self.sounds.registry.iter().all(|(id, _, sound)| {
            let Some(entry) = self.entries.get(id.index()) else {
                return false;
            };
            let ready = |slot: &Slot| matches!(slot, Slot::Ready(_));
            match (&sound.ogg, &entry.recording) {
                (Some(_), Slot::Ready(_)) => true,
                _ => entry.variants.iter().all(ready),
            }
        })
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
        let Some(made) = self.worker.as_ref().map(|worker| worker.made.clone()) else {
            return;
        };
        for (id, part, buffer) in made.try_iter() {
            self.store(id, part, buffer);
        }
        if let Some(since) = self.preparing_since
            && self.is_complete()
        {
            self.preparing_since = None;
            let megabytes = self.cached_samples() as f32 * 4.0 / 1e6;
            let seconds = since.elapsed().as_secs_f32();
            tracing::info!(seconds, megabytes, "every sound made");
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
            Slot::Asked { hurried: true } | Slot::Failed => None,
            Slot::Missing | Slot::Asked { hurried: false } => {
                self.ask(id, part, true);
                None
            }
        }
    }

    /// Asks a buffer; `hurried` when a play waits for it (it may then be made twice, the
    /// first one made is kept).
    fn ask(&mut self, id: SoundId, part: Part, hurried: bool) {
        let Some(sound) = self.sounds.get(id).cloned() else {
            return;
        };
        if self.worker.is_none() {
            self.worker = spawn_worker();
        }
        let job = Job { id, part, sound };
        let job = match &self.worker {
            Some(worker) => match worker.queue(hurried).send(job) {
                Ok(()) => {
                    self.set(id, part, Slot::Asked { hurried });
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
        if matches!(self.slot(id, part), Some(Slot::Ready(_))) {
            return;
        }
        let failed = buffer.is_none();
        self.set(id, part, buffer.map_or(Slot::Failed, Slot::Ready));
        if failed && part == Part::Recording && self.preparing {
            self.ask_variants(id);
        }
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
            bank.sounds
                .registry
                .iter()
                .any(|(id, _, _)| bank.is_coming(id))
        };
        while asked(self) {
            self.collect();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// Samples kept in memory.
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

impl Worker {
    fn queue(&self, hurried: bool) -> &Sender<Job> {
        if hurried {
            &self.urgent
        } else {
            &self.background
        }
    }
}

/// The next job, an urgent one if there is any; `None` once the bank is gone.
fn next_job(urgent: &Receiver<Job>, background: &Receiver<Job>) -> Option<Job> {
    if let Ok(job) = urgent.try_recv() {
        return Some(job);
    }
    crossbeam_channel::select! {
        recv(urgent) -> job => job.ok(),
        recv(background) -> job => job.ok(),
    }
}

/// The thread that makes the buffers; `None` (and the work done in place) without one.
fn spawn_worker() -> Option<Worker> {
    let (urgent, urgent_jobs) = crossbeam_channel::unbounded::<Job>();
    let (background, background_jobs) = crossbeam_channel::unbounded::<Job>();
    let (made_sender, made) = crossbeam_channel::unbounded();
    let spawned = std::thread::Builder::new()
        .name("sound-bank".to_owned())
        .spawn(move || {
            while let Some(job) = next_job(&urgent_jobs, &background_jobs) {
                if made_sender.send(make(job)).is_err() {
                    break;
                }
            }
        });
    match spawned {
        Ok(_) => Some(Worker {
            urgent,
            background,
            made,
        }),
        Err(error) => {
            tracing::warn!(%error, "no sound thread: sounds are made on first play");
            None
        }
    }
}
