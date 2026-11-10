//! Composition: a theme and a seed give a score of piano phrases built from the theme's
//! motifs over a slow walk of chords, sometimes held by a pad, with long rests between.

use super::theme::{Motif, ThemeDef};
use crate::audio::Rng;

/// How the music should feel. Only calm exploration for now; fights (v0.4) will add an
/// intensity that layers more parts over the same score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mood {
    Calm,
}

impl Mood {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Calm => "calm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instrument {
    Piano,
    Pad,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// Seconds from the start of the piece.
    pub start: f32,
    /// Seconds before the note is let go (the piano's damper, the pad's release).
    pub hold: f32,
    /// MIDI number.
    pub pitch: u8,
    /// 0 to 1.
    pub velocity: f32,
    pub instrument: Instrument,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    /// In order of start.
    pub notes: Vec<Note>,
    pub seconds: f32,
}

/// Seconds a let-go piano note still sounds (the damper).
pub const DAMPER: f32 = 0.6;
/// Seconds the reverberation rings after the last note.
pub const ROOM_TAIL: f32 = 2.5;
/// Silence before the first note, at most.
const INTRO: (f32, f32) = (1.0, 4.0);
const MAX_MOTIFS_PER_PHRASE: f32 = 3.0;
/// Chance that a note of a motif is left out (a breath).
const BREATH: f32 = 0.12;

/// Seconds a piano note of `pitch` takes to fade by 60 dB: lower notes ring longer.
pub fn piano_ring(def: &ThemeDef, pitch: u8) -> f32 {
    let ratio = 261.63 / hertz(pitch);
    (def.piano.decay * ratio.powf(0.3)).clamp(0.2, 30.0)
}

pub fn hertz(pitch: u8) -> f32 {
    440.0 * 2f32.powf((f32::from(pitch) - 69.0) / 12.0)
}

impl Note {
    /// When it has faded out, reverberation aside.
    pub fn end(&self, def: &ThemeDef) -> f32 {
        let release = match self.instrument {
            Instrument::Piano => DAMPER,
            Instrument::Pad => def.pad.as_ref().map_or(0.0, |pad| pad.release),
        };
        self.start + self.hold + release
    }
}

pub fn compose(def: &ThemeDef, seed: u64, mood: Mood) -> Score {
    match mood {
        Mood::Calm => calm(def, seed),
    }
}

fn calm(def: &ThemeDef, seed: u64) -> Score {
    let mut rng = Rng::new(seed);
    let beat = 60.0 / def.tempo;
    let target = rng.range(def.length);
    let limit = target - ROOM_TAIL;
    let mut notes = Vec::new();
    let mut at = rng.range(INTRO);
    let mut chord = pick(&def.chords, &mut rng);
    while at < limit {
        let phrase = Phrase {
            def,
            beat,
            limit,
            chord: i32::from(chord),
        }
        .play(at, &mut rng);
        if phrase.is_empty() {
            break;
        }
        let end = phrase.iter().map(|n| n.end(def)).fold(at, f32::max);
        notes.extend(phrase);
        let rest = rng.range(def.rests) * (1.5 - def.density);
        at = end + ROOM_TAIL + rest;
        chord = pick(&def.chords, &mut rng);
    }
    // What would ring past the end is let go earlier.
    for note in &mut notes {
        let over = note.end(def) - limit;
        if over > 0.0 {
            note.hold -= over;
        }
    }
    notes.retain(|note| note.hold > 0.0);
    notes.sort_by(|a, b| a.start.total_cmp(&b.start));
    Score {
        notes,
        seconds: target,
    }
}

struct Phrase<'a> {
    def: &'a ThemeDef,
    beat: f32,
    limit: f32,
    chord: i32,
}

impl Phrase<'_> {
    /// One to three motifs from `start`, maybe a low note and a held chord under them.
    fn play(&self, start: f32, rng: &mut Rng) -> Vec<Note> {
        let def = self.def;
        let mut notes = Vec::new();
        let count = 1 + (rng.unit() * (1.0 + (MAX_MOTIFS_PER_PHRASE - 1.0) * def.density)) as usize;
        let mut at = start;
        for _ in 0..count {
            let motif = varied(&def.motifs[rng.next_u64() as usize % def.motifs.len()], rng);
            if !self.motif(&motif, &mut at, rng, &mut notes) {
                break;
            }
            at += self.beat * f32::from(rng.count((1, 3)));
        }
        if notes.is_empty() {
            return notes;
        }
        let length = (at - start).max(self.beat);
        let melody = i32::from(def.piano.octave);
        if rng.unit() < def.piano.bass {
            let pitch = self.pitch(melody - 1, 0);
            let hold = length.min(piano_ring(def, pitch));
            notes.push(piano(start, hold, pitch, rng.range((0.35, 0.5))));
        }
        if let Some(pad) = &def.pad
            && rng.unit() < pad.chance
        {
            for degree in [0, 2, 4] {
                notes.push(Note {
                    start,
                    hold: length,
                    pitch: self.pitch(i32::from(pad.octave), degree),
                    velocity: rng.range((0.6, 0.8)),
                    instrument: Instrument::Pad,
                });
            }
        }
        notes
    }

    /// Plays a motif from `at`; false if it would run past the end of the piece.
    fn motif(&self, motif: &Motif, at: &mut f32, rng: &mut Rng, notes: &mut Vec<Note>) -> bool {
        let octave = i32::from(self.def.piano.octave);
        for (degree, beats) in motif.degrees.iter().zip(&motif.beats) {
            let length = beats * self.beat;
            if *at + length > self.limit {
                return false;
            }
            if rng.unit() >= BREATH {
                let pitch = self.pitch(octave, i32::from(*degree));
                let ring = piano_ring(self.def, pitch);
                let hold = (length * 2.5).min(ring).max(length.min(ring));
                notes.push(piano(*at, hold, pitch, rng.range((0.45, 0.8))));
            }
            *at += length;
        }
        true
    }

    fn pitch(&self, octave: i32, degree: i32) -> u8 {
        let root = i32::from(self.def.root) + 12 * octave;
        self.def.scale.note(root, self.chord + degree)
    }
}

fn piano(start: f32, hold: f32, pitch: u8, velocity: f32) -> Note {
    Note {
        start,
        hold,
        pitch,
        velocity,
        instrument: Instrument::Piano,
    }
}

/// The motif as written, backwards, upside down, slower, or moved a step.
fn varied(motif: &Motif, rng: &mut Rng) -> Motif {
    let mut motif = motif.clone();
    let roll = rng.unit();
    if roll < 0.15 {
        motif.degrees.reverse();
        motif.beats.reverse();
    } else if roll < 0.3 {
        let first = motif.degrees.first().copied().unwrap_or(0);
        for degree in &mut motif.degrees {
            *degree = first.saturating_mul(2).saturating_sub(*degree);
        }
    } else if roll < 0.45 {
        for beat in &mut motif.beats {
            *beat *= 2.0;
        }
    } else if roll < 0.6 {
        let shift = if rng.unit() < 0.5 { -1 } else { 1 };
        for degree in &mut motif.degrees {
            *degree = degree.saturating_add(shift);
        }
    }
    motif
}

fn pick(values: &[i8], rng: &mut Rng) -> i8 {
    values
        .get(rng.next_u64() as usize % values.len().max(1))
        .copied()
        .unwrap_or(0)
}
