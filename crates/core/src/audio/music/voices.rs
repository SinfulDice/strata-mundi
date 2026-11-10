//! The instruments: a piano (hammer noise, slightly stretched harmonics that fade faster
//! the higher they are, a damper when let go) and a soft pad of detuned sines.
//! Partials are rotating phasors: two multiply-adds per sample, no `sin`.

use super::compose::{DAMPER, Instrument, Note, hertz, piano_ring};
use super::theme::{Pad, ThemeDef};
use crate::audio::SAMPLE_RATE;
use crate::audio::dsp::{Biquad, FilterKind, MAX_HZ, Noise, NoiseColor, Rng};
use std::f64::consts::TAU;

const RATE: f64 = SAMPLE_RATE as f64;
/// Loudness of one note at full velocity, before the theme's volumes.
const PIANO_GAIN: f64 = 0.22;
const PAD_GAIN: f64 = 0.07;
const PIANO_PARTIALS: u32 = 8;
/// Stretch of the piano's harmonics (string stiffness).
const INHARMONICITY: f64 = 0.0004;
const ATTACK_SECONDS: f64 = 0.004;
const HAMMER_SECONDS: f32 = 0.03;

/// −60 dB after `seconds`, as a factor per sample.
fn decay_factor(seconds: f64) -> f64 {
    (-6.907_755 / (seconds.max(1e-3) * RATE)).exp()
}

#[derive(Debug, Clone)]
struct Phasor {
    re: f64,
    im: f64,
    cos: f64,
    sin: f64,
}

impl Phasor {
    fn new(hz: f64, amplitude: f64, decay: f64, phase: f64) -> Self {
        let w = TAU * hz / RATE;
        Self {
            re: amplitude * phase.cos(),
            im: amplitude * phase.sin(),
            cos: decay * w.cos(),
            sin: decay * w.sin(),
        }
    }

    fn next(&mut self) -> f64 {
        let re = self.re * self.cos - self.im * self.sin;
        self.im = self.re * self.sin + self.im * self.cos;
        self.re = re;
        self.im
    }
}

pub(super) struct Voice {
    partials: Vec<Phasor>,
    kind: Kind,
    /// Samples played so far, and when the note is let go and over.
    age: usize,
    hold: usize,
    end: usize,
    gain: f64,
}

enum Kind {
    Piano {
        hammer: Biquad,
        noise: Noise,
        rng: Rng,
        damper: f64,
        released: f64,
    },
    Pad {
        attack: usize,
        release: usize,
        low: f64,
        smoothing: f64,
        tremolo: Phasor,
        tone: f64,
        darkening: f64,
    },
}

impl Voice {
    pub fn new(def: &ThemeDef, note: &Note, seed: u64) -> Option<Self> {
        match note.instrument {
            Instrument::Piano => Some(piano(def, note, seed)),
            Instrument::Pad => def.pad.as_ref().map(|pad| self::pad(pad, note, seed)),
        }
    }

    /// Adds the next samples into `out`; true once the note is over.
    pub fn render(&mut self, out: &mut [f32]) -> bool {
        for sample in out.iter_mut() {
            if self.age >= self.end {
                return true;
            }
            let value: f64 = self.partials.iter_mut().map(Phasor::next).sum();
            let value = self.tone(value) * self.envelope();
            *sample += (value * self.gain) as f32;
            self.age += 1;
        }
        self.age >= self.end
    }

    /// The pad's low-pass; the piano is left as it is.
    fn tone(&mut self, value: f64) -> f64 {
        match &mut self.kind {
            Kind::Piano { .. } => value,
            Kind::Pad {
                tone, darkening, ..
            } => {
                *tone += (value - *tone) * *darkening;
                *tone
            }
        }
    }

    fn envelope(&mut self) -> f64 {
        let (age, hold) = (self.age, self.hold);
        match &mut self.kind {
            Kind::Piano {
                hammer,
                noise,
                rng,
                damper,
                released,
            } => {
                let attack = (age as f64 / (ATTACK_SECONDS * RATE)).min(1.0);
                if age >= hold {
                    *released *= *damper;
                }
                let thump = if (age as f32) < HAMMER_SECONDS * SAMPLE_RATE as f32 {
                    let t = age as f32 / SAMPLE_RATE as f32;
                    f64::from(hammer.process(noise.next(rng)) * (-t / 0.008).exp()) * 0.15
                } else {
                    0.0
                };
                attack * *released + thump
            }
            Kind::Pad {
                attack,
                release,
                low,
                smoothing,
                tremolo,
                ..
            } => {
                let swell = smooth_step(age as f64 / (*attack).max(1) as f64);
                let fade = if age >= hold {
                    1.0 - smooth_step((age - hold) as f64 / (*release).max(1) as f64)
                } else {
                    1.0
                };
                let target = swell * fade * (1.0 + 0.18 * tremolo.next());
                *low += (target - *low) * *smoothing;
                *low
            }
        }
    }
}

fn smooth_step(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn piano(def: &ThemeDef, note: &Note, seed: u64) -> Voice {
    let mut rng = Rng::new(seed);
    let f0 = f64::from(hertz(note.pitch));
    let ring = f64::from(piano_ring(def, note.pitch));
    let brightness = 0.25 + 0.45 * f64::from(def.piano.brightness);
    let velocity = f64::from(note.velocity);
    let mut partials = Vec::new();
    for k in 1..=PIANO_PARTIALS {
        let k = f64::from(k);
        let hz = k * f0 * (1.0 + INHARMONICITY * k * k).sqrt();
        if hz >= f64::from(MAX_HZ) {
            break;
        }
        // Soft notes are darker: the higher partials lose more with velocity.
        let amplitude = brightness.powf(k - 1.0) * velocity.powf(0.4 * (k - 1.0)) / k.sqrt();
        let seconds = ring / (1.0 + 0.55 * (k - 1.0));
        let phase = f64::from(rng.unit()) * TAU;
        partials.push(Phasor::new(hz, amplitude, decay_factor(seconds), phase));
        if k <= 2.0 {
            // A second string slightly out of tune: the slow beating of a real piano.
            let detuned = hz * (1.0 + 0.0007 * k);
            let second = decay_factor(seconds * 0.7);
            partials.push(Phasor::new(detuned, amplitude * 0.5, second, phase));
        }
    }
    let hammer_hz = (f0 * 4.0).min(3000.0) as f32;
    let hold = samples(note.hold);
    Voice {
        partials,
        kind: Kind::Piano {
            hammer: Biquad::new(FilterKind::Band, hammer_hz, 1.2),
            noise: Noise::new(NoiseColor::White),
            rng,
            damper: decay_factor(f64::from(DAMPER)),
            released: 1.0,
        },
        age: 0,
        hold,
        end: hold + samples(DAMPER),
        gain: PIANO_GAIN * velocity * f64::from(def.piano.volume),
    }
}

fn pad(def: &Pad, note: &Note, seed: u64) -> Voice {
    let mut rng = Rng::new(seed);
    let f0 = f64::from(hertz(note.pitch));
    let mut partials = Vec::new();
    for (k, amplitude) in [(1.0, 1.0), (2.0, 0.25), (3.0, 0.08)] {
        for detune in [0.997, 1.003] {
            let hz = f0 * k * detune;
            if hz < f64::from(def.cutoff) * 4.0 {
                let phase = f64::from(rng.unit()) * TAU;
                partials.push(Phasor::new(hz, amplitude, 1.0, phase));
            }
        }
    }
    let lfo = 0.08 + 0.1 * f64::from(rng.unit());
    let hold = samples(note.hold);
    let release = samples(def.release);
    Voice {
        partials,
        kind: Kind::Pad {
            attack: samples(def.attack),
            release,
            low: 0.0,
            smoothing: 1.0 - (-TAU * 30.0 / RATE).exp(),
            tremolo: Phasor::new(lfo, 1.0, 1.0, f64::from(rng.unit()) * TAU),
            tone: 0.0,
            darkening: 1.0 - (-TAU * f64::from(def.cutoff) / RATE).exp(),
        },
        age: 0,
        hold,
        end: hold + release,
        gain: PAD_GAIN * f64::from(note.velocity) * f64::from(def.volume),
    }
}

fn samples(seconds: f32) -> usize {
    (f64::from(seconds.max(0.0)) * RATE) as usize
}
