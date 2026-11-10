//! Reverberation of the place, after Freeverb (Jezar, public domain): eight damped comb
//! filters in parallel then four all-pass filters in series, one set per ear with
//! slightly longer delays on the right. Feedback below 1 and damped: the tail always dies.

use super::space::Space;
use strata_core::audio::{Sounds, SpaceDef};
use strata_core::id::Identifier;

/// Freeverb's delays at 44.1 kHz, in samples.
const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const ALLPASS_FEEDBACK: f32 = 0.5;
const INPUT_GAIN: f32 = 0.015;
/// Scale of the wet signal (Freeverb's 3) once the input gain is accounted for.
const WET_SCALE: f32 = 3.0;
/// Fraction of the way to the target settings travelled per block (no zipper noise).
const GLIDE: f32 = 0.15;
/// Below this wet level (and aiming at it) the reverberation is not computed at all.
const SILENT: f32 = 1e-4;

/// How a place rings (from the pack).
pub type Room = SpaceDef;

/// The pack's room of each space; a space the packs leave out does not ring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rooms([Room; 4]);

impl Rooms {
    pub fn new(sounds: &Sounds) -> Self {
        Self(Space::ALL.map(|space| {
            Identifier::new("strata", space.name())
                .ok()
                .and_then(|id| sounds.spaces.get(&id).copied())
                .unwrap_or(Room::DRY)
        }))
    }

    pub fn of(&self, space: Space) -> Room {
        self.0[space as usize]
    }
}

impl Default for Rooms {
    fn default() -> Self {
        Self([Room::DRY; 4])
    }
}

fn glide(room: &mut Room, target: Room) {
    let step = |from: &mut f32, to: f32| *from += (to - *from) * GLIDE;
    step(&mut room.feedback, target.feedback);
    step(&mut room.damping, target.damping);
    step(&mut room.wet, target.wet);
    if (room.wet - target.wet).abs() < SILENT {
        room.wet = target.wet;
    }
}

/// The next index of a delay line (cheaper than a remainder at every sample).
fn next(index: usize, len: usize) -> usize {
    if index + 1 >= len { 0 } else { index + 1 }
}

struct Comb {
    buffer: Vec<f32>,
    index: usize,
    filtered: f32,
}

impl Comb {
    fn process(&mut self, input: f32, feedback: f32, damping: f32) -> f32 {
        let out = self.buffer[self.index];
        self.filtered = out * (1.0 - damping) + self.filtered * damping;
        self.buffer[self.index] = input + self.filtered * feedback;
        self.index = next(self.index, self.buffer.len());
        out
    }
}

struct AllPass {
    buffer: Vec<f32>,
    index: usize,
}

impl AllPass {
    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.index];
        self.buffer[self.index] = input + delayed * ALLPASS_FEEDBACK;
        self.index = next(self.index, self.buffer.len());
        delayed - input
    }
}

struct Ear {
    combs: Vec<Comb>,
    allpasses: Vec<AllPass>,
}

impl Ear {
    fn new(rate: u32, spread: usize) -> Self {
        let scaled = |samples: usize| ((samples + spread) * rate as usize / 44_100).max(1);
        Self {
            combs: COMBS
                .iter()
                .map(|&d| Comb {
                    buffer: vec![0.0; scaled(d)],
                    index: 0,
                    filtered: 0.0,
                })
                .collect(),
            allpasses: ALLPASSES
                .iter()
                .map(|&d| AllPass {
                    buffer: vec![0.0; scaled(d)],
                    index: 0,
                })
                .collect(),
        }
    }

    fn process(&mut self, input: f32, room: &Room) -> f32 {
        let mut sum = 0.0;
        for comb in &mut self.combs {
            sum += comb.process(input, room.feedback, room.damping);
        }
        self.allpasses
            .iter_mut()
            .fold(sum, |signal, allpass| allpass.process(signal))
    }

    fn clear(&mut self) {
        for comb in &mut self.combs {
            comb.buffer.fill(0.0);
            comb.filtered = 0.0;
        }
        for allpass in &mut self.allpasses {
            allpass.buffer.fill(0.0);
        }
    }
}

pub struct Reverb {
    ears: [Ear; 2],
    room: Room,
    target: Room,
    /// The delay lines hold an old tail, to forget before ringing again.
    stale: bool,
}

impl Reverb {
    pub fn new(rate: u32) -> Self {
        Self {
            ears: [Ear::new(rate, 0), Ear::new(rate, STEREO_SPREAD)],
            room: Room::DRY,
            target: Room::DRY,
            stale: false,
        }
    }

    pub fn set_room(&mut self, room: Room) {
        self.target = room;
    }

    /// Stops at once, the tail forgotten (it must not ring again when turned back on).
    pub fn silence(&mut self) {
        self.target = Room::DRY;
        self.room = self.target;
        self.stale = true;
    }

    /// Whether it rings (or is still dying out).
    pub fn is_active(&self) -> bool {
        self.room.wet >= SILENT || self.target.wet >= SILENT
    }

    /// Adds the reverberation of the mono `send` to `out` (interleaved stereo).
    pub fn render(&mut self, send: &[f32], out: &mut [f32]) {
        if !self.is_active() {
            self.room = self.target;
            self.stale = true;
            return;
        }
        if self.stale {
            self.ears.iter_mut().for_each(Ear::clear);
            self.stale = false;
        }
        let start = self.room.wet;
        glide(&mut self.room, self.target);
        let frames = send.len().min(out.len() / 2);
        for (frame, &input) in send.iter().take(frames).enumerate() {
            let ramp = (frame + 1) as f32 / frames as f32;
            let wet = (start + (self.room.wet - start) * ramp) * WET_SCALE;
            let input = input * INPUT_GAIN;
            for (channel, ear) in self.ears.iter_mut().enumerate() {
                out[frame * 2 + channel] += ear.process(input, &self.room) * wet;
            }
        }
    }
}
