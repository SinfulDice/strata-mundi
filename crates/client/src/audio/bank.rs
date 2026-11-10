//! Buffers of every sound, made the first time each variant is played and kept: rendered
//! from the recipe, or decoded from the pack's `.ogg`.

use std::sync::Arc;
use strata_core::audio::{self, MAX_SECONDS, Rng, SAMPLE_RATE, Sound, Sounds};
use strata_core::content::{RegistryId, SoundId};

/// Longest recording kept (memory bound for packs that ship long files).
const MAX_RECORDING_SECONDS: f32 = 6.0 * MAX_SECONDS;

/// One play of a sound: which buffer and how fast.
#[derive(Debug, Clone)]
pub struct Pick {
    pub buffer: Arc<[f32]>,
    pub pitch: f32,
}

#[derive(Default)]
struct Entry {
    variants: Vec<Option<Arc<[f32]>>>,
    /// `None` until first played; `Some(None)` when it could not be decoded.
    recording: Option<Option<Arc<[f32]>>>,
    last_variant: Option<usize>,
}

#[derive(Default)]
pub struct SoundBank {
    sounds: Sounds,
    entries: Vec<Entry>,
}

impl SoundBank {
    pub fn new(sounds: Sounds) -> Self {
        let entries = sounds
            .registry
            .iter()
            .map(|(_, _, sound)| Entry {
                variants: vec![None; usize::from(sound.def.variants)],
                ..Entry::default()
            })
            .collect();
        Self { sounds, entries }
    }

    pub fn sound(&self, id: SoundId) -> Option<&Sound> {
        self.sounds.get(id)
    }

    /// A buffer and a pitch for one play; never the same variant twice in a row.
    pub fn pick(&mut self, id: SoundId, rng: &mut Rng) -> Option<Pick> {
        let sound = self.sounds.get(id)?;
        let entry = self.entries.get_mut(id.index())?;
        let pitch = rng.range(sound.def.pitch);
        if let Some(path) = &sound.ogg {
            let recording = entry.recording.get_or_insert_with(|| decode(path));
            if let Some(buffer) = recording {
                return Some(Pick {
                    buffer: Arc::clone(buffer),
                    pitch,
                });
            }
        }
        let count = entry.variants.len();
        let mut index = (rng.next_u64() % count.max(1) as u64) as usize;
        if count > 1 && entry.last_variant == Some(index) {
            index = (index + 1) % count;
        }
        entry.last_variant = Some(index);
        let slot = entry.variants.get_mut(index)?;
        let buffer = slot.get_or_insert_with(|| {
            let seed = audio::variant_seed(sound, index as u8);
            audio::render(&sound.def, seed).into()
        });
        Some(Pick {
            buffer: Arc::clone(buffer),
            pitch,
        })
    }

    /// Samples kept in memory.
    #[cfg(test)]
    pub fn cached_samples(&self) -> usize {
        self.entries
            .iter()
            .flat_map(|entry| entry.variants.iter().chain(entry.recording.iter()))
            .flatten()
            .map(|buffer| buffer.len())
            .sum()
    }
}

/// Mono at `SAMPLE_RATE`; `None` (and a warning) if the file cannot be read.
fn decode(path: &std::path::Path) -> Option<Arc<[f32]>> {
    let read = || -> Result<Vec<f32>, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let decoder = rodio::Decoder::try_from(file).map_err(|e| e.to_string())?;
        let channels = usize::from(rodio::Source::channels(&decoder).get());
        let rate = rodio::Source::sample_rate(&decoder).get();
        let limit = (MAX_RECORDING_SECONDS * rate as f32) as usize * channels;
        let interleaved: Vec<f32> = decoder.take(limit).collect();
        Ok(resample(&downmix(&interleaved, channels), rate))
    };
    match read() {
        Ok(samples) if !samples.is_empty() => Some(samples.into()),
        Ok(_) => {
            tracing::warn!(path = %path.display(), "empty recording: the recipe is used");
            None
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "recording unreadable: the recipe is used");
            None
        }
    }
}

fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    interleaved
        .chunks(channels.max(1))
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Linear resampling to `SAMPLE_RATE` (recordings are short effects).
pub fn resample(samples: &[f32], rate: u32) -> Vec<f32> {
    if rate == SAMPLE_RATE || samples.len() < 2 {
        return samples.to_vec();
    }
    let ratio = f64::from(rate) / f64::from(SAMPLE_RATE);
    let len = (samples.len() as f64 / ratio) as usize;
    (0..len)
        .map(|i| {
            let position = i as f64 * ratio;
            let index = (position as usize).min(samples.len() - 2);
            let fraction = (position - index as f64) as f32;
            samples[index] + (samples[index + 1] - samples[index]) * fraction
        })
        .collect()
}
