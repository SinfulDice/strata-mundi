//! Rendering a score block by block (a few kilobytes at a time, never the whole piece in
//! memory): notes start as their time comes, a small room reverberates them, a gentle
//! limiter keeps every sample below 1.

use super::compose::Score;
use super::theme::{Room, ThemeDef};
use super::voices::Voice;
use crate::audio::SAMPLE_RATE;

/// Seconds faded out at the very end, in case something still rings.
const FADE_OUT: f32 = 1.5;
/// Below this the limiter does nothing.
const KNEE: f32 = 0.7;

pub struct PieceRenderer {
    def: ThemeDef,
    score: Score,
    next: usize,
    playing: Vec<(usize, Voice)>,
    room: RoomVerb,
    frame: usize,
    frames: usize,
    seed: u64,
}

impl PieceRenderer {
    pub fn new(def: &ThemeDef, score: Score, seed: u64) -> Self {
        let frames = (score.seconds.max(0.0) * SAMPLE_RATE as f32) as usize;
        Self {
            room: RoomVerb::new(&def.room),
            def: def.clone(),
            score,
            next: 0,
            playing: Vec::new(),
            frame: 0,
            frames,
            seed,
        }
    }

    /// Length of the piece in samples.
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Writes the next samples into `out`; how many were written (0 once finished).
    pub fn fill(&mut self, out: &mut [f32]) -> usize {
        let count = out.len().min(self.frames - self.frame);
        let out = &mut out[..count];
        out.fill(0.0);
        self.start_notes(count);
        self.playing.retain_mut(|(offset, voice)| {
            let start = (*offset).min(count);
            *offset = 0;
            !voice.render(&mut out[start..])
        });
        let fade_from = self
            .frames
            .saturating_sub((FADE_OUT * SAMPLE_RATE as f32) as usize);
        let volume = self.def.volume;
        for (i, sample) in out.iter_mut().enumerate() {
            let wet = self.room.process(*sample);
            let frame = self.frame + i;
            let fade = if frame >= fade_from {
                (self.frames - frame) as f32 / (self.frames - fade_from).max(1) as f32
            } else {
                1.0
            };
            *sample = limit(wet * volume * fade);
        }
        self.frame += count;
        count
    }

    fn start_notes(&mut self, count: usize) {
        let end = self.frame + count;
        while let Some(note) = self.score.notes.get(self.next) {
            let start = (note.start.max(0.0) * SAMPLE_RATE as f32) as usize;
            if start >= end {
                break;
            }
            let seed = self.seed ^ (self.next as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            if let Some(voice) = Voice::new(&self.def, note, seed) {
                self.playing.push((start.saturating_sub(self.frame), voice));
            }
            self.next += 1;
        }
    }
}

/// The whole piece at once (tests and the `--render-music` tool).
pub fn render_piece(def: &ThemeDef, score: Score, seed: u64) -> Vec<f32> {
    let mut renderer = PieceRenderer::new(def, score, seed);
    let mut samples = vec![0.0; renderer.frames()];
    let mut written = 0;
    while written < samples.len() {
        let count = renderer.fill(&mut samples[written..]);
        if count == 0 {
            break;
        }
        written += count;
    }
    samples
}

/// Linear up to the knee, then a soft curve that never reaches 1.
fn limit(sample: f32) -> f32 {
    let size = sample.abs();
    if size <= KNEE {
        return sample;
    }
    let room = 1.0 - KNEE;
    let over = KNEE + room * ((size - KNEE) / room).tanh();
    over.min(0.999).copysign(sample)
}

/// A small Schroeder reverberation: four damped combs, two all-passes.
struct RoomVerb {
    combs: Vec<Comb>,
    passes: Vec<AllPass>,
    wet: f32,
}

const COMBS: [usize; 4] = [1557, 1617, 1491, 1422];
const PASSES: [usize; 2] = [556, 225];
const INPUT: f32 = 0.12;

impl RoomVerb {
    fn new(room: &Room) -> Self {
        let scale = 0.6 + 0.6 * room.size;
        let feedback = 0.7 + 0.24 * room.size;
        Self {
            combs: COMBS
                .iter()
                .map(|&len| Comb::new((len as f32 * scale) as usize, feedback, 0.3))
                .collect(),
            passes: PASSES.iter().map(|&len| AllPass::new(len)).collect(),
            wet: room.wet,
        }
    }

    fn process(&mut self, dry: f32) -> f32 {
        if self.wet <= 0.0 {
            return dry;
        }
        let input = dry * INPUT;
        let mut wet: f32 = self.combs.iter_mut().map(|comb| comb.process(input)).sum();
        for pass in &mut self.passes {
            wet = pass.process(wet);
        }
        dry * (1.0 - 0.5 * self.wet) + wet * self.wet
    }
}

struct Comb {
    line: Vec<f32>,
    index: usize,
    feedback: f32,
    damping: f32,
    low: f32,
}

impl Comb {
    fn new(len: usize, feedback: f32, damping: f32) -> Self {
        Self {
            line: vec![0.0; len.max(1)],
            index: 0,
            feedback,
            damping,
            low: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let out = self.line[self.index];
        self.low = out * (1.0 - self.damping) + self.low * self.damping;
        self.line[self.index] = input + self.low * self.feedback;
        self.index = (self.index + 1) % self.line.len();
        out
    }
}

struct AllPass {
    line: Vec<f32>,
    index: usize,
}

impl AllPass {
    fn new(len: usize) -> Self {
        Self {
            line: vec![0.0; len.max(1)],
            index: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let delayed = self.line[self.index];
        self.line[self.index] = input + delayed * 0.5;
        self.index = (self.index + 1) % self.line.len();
        delayed - input
    }
}
