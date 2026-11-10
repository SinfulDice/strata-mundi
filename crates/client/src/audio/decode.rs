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
        let mut mono = Vec::new();
        downmix_into(&interleaved, channels, &mut mono);
        Ok(resample(&mono, rate))
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

/// Replaces the content of `mono` with the mean of each frame of `interleaved`.
pub fn downmix_into(interleaved: &[f32], channels: usize, mono: &mut Vec<f32>) {
    mono.clear();
    mono.extend(
        interleaved
            .chunks(channels.max(1))
            .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32),
    );
}

/// Linear resampling of a whole recording to `SAMPLE_RATE`.
pub fn resample(samples: &[f32], rate: u32) -> Vec<f32> {
    let mut out = Vec::new();
    Resampler::new(rate).push(samples, &mut out);
    out
}

/// Linear resampling to `SAMPLE_RATE` of a stream read block by block: the position and
/// the last sample are carried from one block to the next (no click, no drift).
#[derive(Debug, Clone)]
pub struct Resampler {
    /// Input samples per output sample.
    ratio: f64,
    /// Where the next output sample falls, counted from the carried sample.
    position: f64,
    last: Option<f32>,
}

impl Resampler {
    pub fn new(rate: u32) -> Self {
        Self {
            ratio: f64::from(rate.max(1)) / f64::from(SAMPLE_RATE),
            position: 0.0,
            last: None,
        }
    }

    /// Adds to `out` the samples of `input` (the next block of the stream).
    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if self.ratio == 1.0 {
            out.extend_from_slice(input);
            return;
        }
        let carried = usize::from(self.last.is_some());
        let len = input.len() + carried;
        let last = self.last.unwrap_or(0.0);
        let at = |index: usize| match index.checked_sub(carried) {
            Some(index) => input[index],
            None => last,
        };
        if len == 0 {
            return;
        }
        loop {
            let index = self.position as usize;
            if index + 1 >= len {
                break;
            }
            let fraction = (self.position - index as f64) as f32;
            let (a, b) = (at(index), at(index + 1));
            out.push(a + (b - a) * fraction);
            self.position += self.ratio;
        }
        self.position -= (len - 1) as f64;
        self.last = Some(at(len - 1));
    }
}
