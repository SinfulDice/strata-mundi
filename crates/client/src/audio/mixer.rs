//! The mixer: sums the playing voices into stereo, each placed relative to the listener
//! and scaled by its category volume. Pure (no device): the output thread only asks it for
//! blocks of samples, and tests read them directly.

use super::spatial::{Listener, stereo_gains};
use crate::settings::SoundSettings;
use glam::DVec3;
use std::sync::Arc;
use strata_core::audio::{SAMPLE_RATE, SoundCategory};

/// Most sounds heard at once; a new one replaces the quietest if it is louder.
pub const MAX_VOICES: usize = 32;

/// A sound to play: a rendered (or decoded) mono buffer at `SAMPLE_RATE`.
#[derive(Debug, Clone)]
pub struct Voice {
    pub buffer: Arc<[f32]>,
    pub category: SoundCategory,
    /// Loudness of this play, before distance and volumes.
    pub gain: f32,
    /// Playback speed (pitch); 1 is as rendered.
    pub pitch: f32,
    /// Where it sounds from; `None`: inside the head (music, interface).
    pub at: Option<DVec3>,
    /// Blocks beyond which it is silent.
    pub range: f32,
}

/// Amplitude factor of each category, master included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volumes([f32; 3]);

impl Volumes {
    pub fn of(settings: &SoundSettings) -> Self {
        Self(SoundCategory::ALL.map(|category| settings.gain(category)))
    }

    fn get(&self, category: SoundCategory) -> f32 {
        self.0[category as usize]
    }
}

impl Default for Volumes {
    fn default() -> Self {
        Self::of(&SoundSettings::default())
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Play(Voice),
    Listener(Listener),
    Volumes(Volumes),
}

struct Playing {
    voice: Voice,
    /// Position in the buffer, in (fractional) samples.
    cursor: f64,
    /// Gains used at the end of the last block, ramped from to avoid clicks.
    gains: [f32; 2],
}

pub struct Mixer {
    playing: Vec<Playing>,
    listener: Listener,
    volumes: Volumes,
    /// Samples of a buffer read per output frame at pitch 1.
    rate_ratio: f64,
}

impl Mixer {
    pub fn new(output_rate: u32) -> Self {
        Self {
            playing: Vec::with_capacity(MAX_VOICES),
            listener: Listener::default(),
            volumes: Volumes::default(),
            rate_ratio: f64::from(SAMPLE_RATE) / f64::from(output_rate.max(1)),
        }
    }

    pub fn voices(&self) -> usize {
        self.playing.len()
    }

    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Play(voice) => self.play(voice),
            Command::Listener(listener) => self.listener = listener,
            Command::Volumes(volumes) => self.volumes = volumes,
        }
    }

    fn play(&mut self, voice: Voice) {
        let gains = self.target(&voice);
        let loudness = gains[0].max(gains[1]);
        if loudness <= 0.0 || voice.buffer.is_empty() {
            return;
        }
        let playing = Playing {
            voice,
            cursor: 0.0,
            gains,
        };
        if self.playing.len() < MAX_VOICES {
            self.playing.push(playing);
            return;
        }
        let quietest = self
            .playing
            .iter()
            .enumerate()
            .map(|(index, p)| (index, p.gains[0].max(p.gains[1])))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((index, quiet)) = quietest
            && quiet < loudness
        {
            self.playing[index] = playing;
        }
    }

    fn target(&self, voice: &Voice) -> [f32; 2] {
        let volume = self.volumes.get(voice.category) * voice.gain;
        match voice.at {
            Some(at) => stereo_gains(&self.listener, at, voice.range).map(|g| g * volume),
            None => [volume * std::f32::consts::FRAC_1_SQRT_2; 2],
        }
    }

    /// Fills `out` (interleaved left, right) with the next frames; finished voices go.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let frames = out.len() / 2;
        let mut index = 0;
        while index < self.playing.len() {
            let target = self.target(&self.playing[index].voice);
            let playing = &mut self.playing[index];
            let finished = mix_voice(playing, target, out, frames, self.rate_ratio);
            if finished {
                self.playing.swap_remove(index);
            } else {
                index += 1;
            }
        }
        for sample in out.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
}

/// Adds one voice to `out`, its gains ramped from the last block's to `target`.
fn mix_voice(
    playing: &mut Playing,
    target: [f32; 2],
    out: &mut [f32],
    frames: usize,
    rate_ratio: f64,
) -> bool {
    let buffer = &playing.voice.buffer;
    let step = rate_ratio * f64::from(playing.voice.pitch.max(0.01));
    let last = buffer.len() - 1;
    let start = playing.gains;
    for frame in 0..frames {
        let position = playing.cursor;
        let index = position as usize;
        if index >= last {
            playing.cursor = buffer.len() as f64;
            break;
        }
        let fraction = (position - index as f64) as f32;
        let sample = buffer[index] + (buffer[index + 1] - buffer[index]) * fraction;
        let ramp = (frame + 1) as f32 / frames as f32;
        for channel in 0..2 {
            let gain = start[channel] + (target[channel] - start[channel]) * ramp;
            out[frame * 2 + channel] += sample * gain;
        }
        playing.cursor += step;
    }
    playing.gains = target;
    playing.cursor as usize >= last
}
