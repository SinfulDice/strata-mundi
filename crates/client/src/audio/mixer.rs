//! The mixer: sums the playing voices, the ambience beds and the music into stereo, each
//! voice placed relative to the listener, muffled behind walls and sent to the
//! reverberation of the place (advanced spatial audio), all scaled by their category
//! volume. Pure (no device): the output thread only asks it for blocks of samples, and
//! tests read them directly.

use super::beds::{BED_SLOTS, Bed, BedVolume};
use super::reverb::Reverb;
use super::space::Space;
use super::spatial::{Listener, stereo_gains};
use super::stream::{MusicCommand, Stream};
use super::trash::Trash;
use crate::settings::SoundSettings;
use glam::DVec3;
use std::sync::Arc;
use strata_core::audio::{SAMPLE_RATE, SoundCategory};

/// Most sounds heard at once; a new one replaces the quietest if it is louder.
pub const MAX_VOICES: usize = 32;
/// Longest block rendered without the reverberation feed growing (no allocation).
const MAX_BLOCK_FRAMES: usize = 8192;

/// A sound to play: a rendered (or decoded) mono buffer at `SAMPLE_RATE`.
#[derive(Debug, Clone)]
pub struct Voice {
    /// Given by the game, to muffle it later as the listener moves.
    pub id: u32,
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
    /// 0 (heard clearly) to 1 (behind thick walls).
    pub muffle: f32,
}

/// Amplitude factor of each category, master included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volumes([f32; 3]);

impl Volumes {
    pub fn of(settings: &SoundSettings) -> Self {
        Self(SoundCategory::ALL.map(|category| settings.gain(category)))
    }

    pub(super) fn from_gains(gains: [f32; 3]) -> Self {
        Self(gains)
    }

    pub(super) fn get(&self, category: SoundCategory) -> f32 {
        self.0[category as usize]
    }
}

impl Default for Volumes {
    fn default() -> Self {
        Self::of(&SoundSettings::default())
    }
}

#[derive(Debug)]
pub enum Command {
    Play(Voice),
    Listener(Listener),
    Volumes(Volumes),
    /// A looping ambience bed, faded to `gain` (0 lets it go).
    Bed(BedVolume),
    Muffle {
        id: u32,
        muffle: f32,
    },
    Space(Space),
    /// Advanced spatial audio (reverberation, muffling) or plain stereo.
    Spatial(bool),
    Music(MusicCommand),
}

/// Cutoff of the muffling low-pass, fully open and behind thick walls.
const OPEN_HZ: f32 = 20_000.0;
const MUFFLED_HZ: f32 = 350.0;
/// Loudness lost when fully muffled.
const MUFFLED_LOSS: f32 = 0.75;

struct Playing {
    voice: Voice,
    /// Position in the buffer, in (fractional) samples.
    cursor: f64,
    /// Gains used at the end of the last block, ramped from to avoid clicks.
    gains: [f32; 2],
    /// Muffling used at the end of the last block, and the low-pass memory.
    muffle: f32,
    low: f32,
}

pub struct Mixer {
    playing: Vec<Playing>,
    beds: Vec<Bed>,
    listener: Listener,
    volumes: Volumes,
    /// Samples of a buffer read per output frame at pitch 1.
    rate_ratio: f64,
    rate: f32,
    spatial: bool,
    reverb: Reverb,
    send: Vec<f32>,
    music: Option<Stream>,
    trash: Trash,
}

impl Mixer {
    pub fn new(output_rate: u32) -> Self {
        let rate = output_rate.max(1);
        Self {
            playing: Vec::with_capacity(MAX_VOICES),
            beds: Vec::with_capacity(BED_SLOTS),
            listener: Listener::default(),
            volumes: Volumes::default(),
            rate_ratio: f64::from(SAMPLE_RATE) / f64::from(rate),
            rate: rate as f32,
            spatial: true,
            reverb: Reverb::new(rate),
            send: Vec::with_capacity(MAX_BLOCK_FRAMES),
            music: None,
            trash: Trash::default(),
        }
    }

    /// What it lets go is freed by whoever empties the trash.
    pub(super) fn with_trash(output_rate: u32, trash: Trash) -> Self {
        Self {
            trash,
            ..Self::new(output_rate)
        }
    }

    pub fn voices(&self) -> usize {
        self.playing.len()
    }

    pub fn beds(&self) -> usize {
        self.beds.len()
    }

    pub fn music_playing(&self) -> bool {
        self.music.is_some()
    }

    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Play(voice) => self.play(voice),
            Command::Listener(listener) => self.listener = listener,
            Command::Volumes(volumes) => self.volumes = volumes,
            Command::Bed(bed) => Bed::set(&mut self.beds, bed, &self.trash),
            Command::Muffle { id, muffle } => {
                if let Some(playing) = self.playing.iter_mut().find(|p| p.voice.id == id) {
                    playing.voice.muffle = muffle.clamp(0.0, 1.0);
                }
            }
            Command::Space(space) => self.reverb.set_space(space),
            Command::Spatial(spatial) => {
                self.spatial = spatial;
                if !spatial {
                    self.reverb.silence();
                }
            }
            Command::Music(music) => Stream::apply(&mut self.music, music, &self.trash),
        }
    }

    fn play(&mut self, voice: Voice) {
        let gains = self.target(&voice);
        let loudness = gains[0].max(gains[1]);
        if loudness <= 0.0 || voice.buffer.is_empty() {
            self.trash.buffer(voice.buffer);
            return;
        }
        let muffle = self.muffle_of(&voice);
        let playing = Playing {
            voice,
            cursor: 0.0,
            gains,
            muffle,
            low: 0.0,
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
        match quietest {
            Some((index, quiet)) if quiet < loudness => {
                let old = std::mem::replace(&mut self.playing[index], playing);
                self.trash.buffer(old.voice.buffer);
            }
            _ => self.trash.buffer(playing.voice.buffer),
        }
    }

    fn muffle_of(&self, voice: &Voice) -> f32 {
        if self.spatial && voice.at.is_some() {
            voice.muffle
        } else {
            0.0
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
        let reverb = self.spatial && self.reverb.is_active();
        self.send.clear();
        self.send.resize(if reverb { frames } else { 0 }, 0.0);
        let mut index = 0;
        while index < self.playing.len() {
            let target = self.target(&self.playing[index].voice);
            let muffle = self.muffle_of(&self.playing[index].voice);
            let mix = Mix {
                out: &mut *out,
                send: &mut self.send,
                frames,
                rate_ratio: self.rate_ratio,
                rate: self.rate,
            };
            if mix.voice(&mut self.playing[index], target, muffle) {
                let over = self.playing.swap_remove(index);
                self.trash.buffer(over.voice.buffer);
            } else {
                index += 1;
            }
        }
        Bed::mix_all(
            &mut self.beds,
            &self.volumes,
            out,
            self.rate_ratio,
            &self.trash,
        );
        let rates = (self.rate_ratio, self.rate);
        Stream::mix(&mut self.music, &self.volumes, out, rates, &self.trash);
        if self.spatial {
            self.reverb.render(&self.send, out);
        }
        for sample in out.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
}

struct Mix<'a> {
    out: &'a mut [f32],
    /// Mono feed of the reverberation (empty when it is off).
    send: &'a mut [f32],
    frames: usize,
    rate_ratio: f64,
    rate: f32,
}

impl Mix<'_> {
    /// Adds one voice, its gains and muffling ramped from the last block's to the targets.
    fn voice(self, playing: &mut Playing, target: [f32; 2], muffle: f32) -> bool {
        let buffer = &playing.voice.buffer;
        let step = self.rate_ratio * f64::from(playing.voice.pitch.max(0.01));
        let last = buffer.len() - 1;
        let start = playing.gains;
        let (from, to) = (playing.muffle, muffle);
        let filtered = from > 0.0 || to > 0.0;
        let hz = OPEN_HZ * (MUFFLED_HZ / OPEN_HZ).powf(from.max(to));
        let coefficient = 1.0 - (-std::f32::consts::TAU * hz / self.rate).exp();
        let frames = self.frames;
        for frame in 0..frames {
            let position = playing.cursor;
            let index = position as usize;
            if index >= last {
                playing.cursor = buffer.len() as f64;
                break;
            }
            let fraction = (position - index as f64) as f32;
            let mut sample = buffer[index] + (buffer[index + 1] - buffer[index]) * fraction;
            let ramp = (frame + 1) as f32 / frames as f32;
            if filtered {
                playing.low += (sample - playing.low) * coefficient;
                let muffle = from + (to - from) * ramp;
                sample = playing.low * (1.0 - MUFFLED_LOSS * muffle);
            } else {
                playing.low = sample;
            }
            let mut heard = 0.0;
            for channel in 0..2 {
                let gain = start[channel] + (target[channel] - start[channel]) * ramp;
                self.out[frame * 2 + channel] += sample * gain;
                heard += gain * 0.5;
            }
            if let Some(send) = self.send.get_mut(frame)
                && playing.voice.at.is_some()
            {
                *send += sample * heard;
            }
            playing.cursor += step;
        }
        playing.gains = target;
        playing.muffle = muffle;
        playing.cursor as usize >= last
    }
}
