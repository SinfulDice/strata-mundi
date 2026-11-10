//! Renders a recipe into a mono buffer at `SAMPLE_RATE`: the layers are summed, the
//! result normalised to the recipe's volume and faded at the end. Deterministic for a seed.

use super::SAMPLE_RATE;
use super::dsp::{Biquad, FilterKind, Noise, NoiseColor, Rng, exp_decay, samples};
use super::recipe::{Band, Envelope, Filter, Layer, Mode, SoundDef, Wobble};
use std::f32::consts::TAU;

/// Samples between two retunings of a moving filter.
const RETUNE_EVERY: usize = 16;
/// Fade at the end of every buffer, so a cut ring never clicks.
const END_FADE_SECONDS: f32 = 0.005;
/// Trailing samples quieter than this (relative to the peak) are dropped.
const SILENCE: f32 = 1e-4;

/// One variant of `def`; another `seed` gives another variant.
pub fn render(def: &SoundDef, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let mut buffer = vec![0.0; samples(def.duration).max(1)];
    for layer in &def.layers {
        add_layer(&mut buffer, layer, &mut rng);
    }
    finish(&mut buffer, def.volume.clamp(0.0, 1.0));
    buffer
}

fn add_layer(buffer: &mut [f32], layer: &Layer, rng: &mut Rng) {
    match layer {
        Layer::Noise {
            color,
            filter,
            envelope,
            gain,
            delay,
            wobble,
        } => {
            let start = samples(rng.range(*delay));
            let shape = Shape::pick(envelope, rng);
            let filter = Tuned::pick(filter, rng);
            let wobble = wobble.map(|w| Lfo::pick(w, rng));
            let burst = Burst {
                start,
                shape,
                gain: *gain,
                color: *color,
            };
            burst.add(buffer, filter, wobble, rng);
        }
        Layer::Grains {
            count,
            spread,
            length,
            color,
            filter,
            gain,
            fade,
            delay,
        } => {
            let start = rng.range(*delay);
            for _ in 0..rng.count(*count) {
                let at = rng.unit() * spread;
                let progress = if *spread > 0.0 { at / spread } else { 0.0 };
                let loudness = gain * (1.0 + (fade - 1.0) * progress) * rng.range((0.4, 1.0));
                let burst = Burst {
                    start: samples(start + at),
                    shape: Shape::grain(rng.range(*length)),
                    gain: loudness,
                    color: *color,
                };
                burst.add(buffer, Tuned::pick(filter, rng), None, rng);
            }
        }
        Layer::Modes { modes, gain, delay } => {
            let start = samples(rng.range(*delay));
            for mode in modes {
                add_mode(buffer, start, mode, *gain, rng);
            }
        }
        Layer::Bubbles {
            count,
            spread,
            hz,
            rise,
            decay,
            gain,
            delay,
        } => {
            let start = rng.range(*delay);
            for _ in 0..rng.count(*count) {
                let bubble = Bubble {
                    start: samples(start + rng.unit() * spread),
                    hz: rng.range(*hz),
                    rise: *rise,
                    decay: rng.range(*decay),
                    gain: gain * rng.range((0.3, 1.0)),
                };
                bubble.add(buffer, rng);
            }
        }
    }
}

/// Attack, hold and exponential decay, in seconds.
#[derive(Debug, Clone, Copy)]
struct Shape {
    attack: f32,
    hold: f32,
    decay: f32,
}

impl Shape {
    fn pick(envelope: &Envelope, rng: &mut Rng) -> Self {
        Self {
            attack: envelope.attack,
            hold: rng.range(envelope.hold),
            decay: rng.range(envelope.decay),
        }
    }

    fn grain(length: f32) -> Self {
        Self {
            attack: 0.0005,
            hold: 0.0,
            decay: length,
        }
    }

    fn length(self) -> f32 {
        self.attack + self.hold + self.decay
    }

    fn at(self, t: f32) -> f32 {
        if t < self.attack {
            t / self.attack
        } else if t < self.attack + self.hold {
            1.0
        } else {
            exp_decay(t - self.attack - self.hold, self.decay)
        }
    }
}

/// A filter with its frequency already picked.
struct Tuned {
    biquad: Option<Biquad>,
    hz: f32,
    sweep: f32,
}

impl Tuned {
    fn pick(filter: &Filter, rng: &mut Rng) -> Self {
        let make = |kind, band: &Band, rng: &mut Rng| {
            let hz = rng.range(band.hz);
            Self {
                biquad: Some(Biquad::new(kind, hz, band.q)),
                hz,
                sweep: band.sweep,
            }
        };
        match filter {
            Filter::None => Self {
                biquad: None,
                hz: 0.0,
                sweep: 1.0,
            },
            Filter::LowPass(band) => make(FilterKind::Low, band, rng),
            Filter::HighPass(band) => make(FilterKind::High, band, rng),
            Filter::BandPass(band) => make(FilterKind::Band, band, rng),
        }
    }
}

/// Two slow sines at unrelated rates: a swell that never quite repeats.
struct Lfo {
    rate: f32,
    depth: f32,
    phases: [f32; 2],
}

impl Lfo {
    fn pick(wobble: Wobble, rng: &mut Rng) -> Self {
        Self {
            rate: wobble.rate,
            depth: wobble.depth,
            phases: [rng.unit() * TAU, rng.unit() * TAU],
        }
    }

    /// Between −1 and 1.
    fn at(&self, t: f32) -> f32 {
        let w = TAU * self.rate * t;
        0.6 * (w + self.phases[0]).sin() + 0.4 * (w * 1.618 + self.phases[1]).sin()
    }
}

struct Burst {
    start: usize,
    shape: Shape,
    gain: f32,
    color: NoiseColor,
}

impl Burst {
    fn add(&self, buffer: &mut [f32], mut filter: Tuned, wobble: Option<Lfo>, rng: &mut Rng) {
        let mut noise = Noise::new(self.color);
        let length = samples(self.shape.length()).max(1);
        let end = (self.start + length).min(buffer.len());
        let rate = SAMPLE_RATE as f32;
        for (i, sample) in buffer[self.start.min(end)..end].iter_mut().enumerate() {
            let t = i as f32 / rate;
            let swell = wobble.as_ref().map_or(0.0, |lfo| lfo.at(t));
            let depth = wobble.as_ref().map_or(0.0, |lfo| lfo.depth);
            if let Some(biquad) = filter.biquad.as_mut().filter(|_| i % RETUNE_EVERY == 0) {
                let progress = i as f32 / length as f32;
                biquad.tune(filter.hz * filter.sweep.powf(progress) * 2f32.powf(depth * swell));
            }
            let raw = noise.next(rng);
            let value = filter.biquad.as_mut().map_or(raw, |b| b.process(raw));
            let loudness = 1.0 - depth * 0.5 * (1.0 - swell);
            *sample += value * self.shape.at(t) * self.gain * loudness;
        }
    }
}

fn add_mode(buffer: &mut [f32], start: usize, mode: &Mode, gain: f32, rng: &mut Rng) {
    let hz = rng.range(mode.hz);
    let decay = rng.range(mode.decay);
    let phase = rng.unit() * TAU;
    let shape = Shape {
        attack: 0.0008,
        hold: 0.0,
        decay,
    };
    let end = (start + samples(shape.length())).min(buffer.len());
    let step = TAU * hz / SAMPLE_RATE as f32;
    for (i, sample) in buffer[start.min(end)..end].iter_mut().enumerate() {
        let t = i as f32 / SAMPLE_RATE as f32;
        *sample += (phase + step * i as f32).sin() * shape.at(t) * gain * mode.gain;
    }
}

struct Bubble {
    start: usize,
    hz: f32,
    rise: f32,
    decay: f32,
    gain: f32,
}

impl Bubble {
    fn add(&self, buffer: &mut [f32], rng: &mut Rng) {
        let shape = Shape {
            attack: 0.001,
            hold: 0.0,
            decay: self.decay,
        };
        let length = samples(shape.length()).max(1);
        let end = (self.start + length).min(buffer.len());
        let mut phase = rng.unit() * TAU;
        for (i, sample) in buffer[self.start.min(end)..end].iter_mut().enumerate() {
            let progress = i as f32 / length as f32;
            let hz = self.hz * (1.0 + (self.rise - 1.0) * progress);
            phase += TAU * hz / SAMPLE_RATE as f32;
            let t = i as f32 / SAMPLE_RATE as f32;
            *sample += phase.sin() * shape.at(t) * self.gain;
        }
    }
}

/// Peak at `volume`, faded end, trailing silence dropped.
fn finish(buffer: &mut Vec<f32>, volume: f32) {
    let peak = buffer.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
    if peak < 1e-9 || !peak.is_finite() {
        buffer.iter_mut().for_each(|s| *s = 0.0);
        return;
    }
    let scale = volume / peak;
    buffer.iter_mut().for_each(|s| *s *= scale);
    let audible = buffer
        .iter()
        .rposition(|s| s.abs() > SILENCE * volume.max(1e-6))
        .map_or(1, |last| last + 1);
    buffer.truncate(audible);
    let fade = samples(END_FADE_SECONDS).min(buffer.len());
    let len = buffer.len();
    for (i, sample) in buffer[len - fade..].iter_mut().enumerate() {
        *sample *= 1.0 - (i + 1) as f32 / fade as f32;
    }
}
