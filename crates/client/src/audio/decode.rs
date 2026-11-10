//! Recordings of the packs, made mono at `SAMPLE_RATE`.

use std::sync::Arc;
use strata_core::audio::{MAX_SECONDS, SAMPLE_RATE};

/// Longest recording kept (memory bound for packs that ship long files).
const MAX_RECORDING_SECONDS: f32 = 6.0 * MAX_SECONDS;

/// Mono at `SAMPLE_RATE`; `None` (and a warning) if the file cannot be read.
pub fn decode(path: &std::path::Path) -> Option<Arc<[f32]>> {
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

pub fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
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
