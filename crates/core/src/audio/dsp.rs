//! Building blocks of the synthesis: a seeded generator, coloured noise, biquad filters.

use super::SAMPLE_RATE;
use std::f32::consts::TAU;

/// SplitMix64: tiny, fast, and the same sequence on every machine for a seed.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in [-1, 1).
    pub fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    pub fn range(&mut self, (a, b): (f32, f32)) -> f32 {
        a + (b - a) * self.unit()
    }

    pub fn count(&mut self, (a, b): (u16, u16)) -> u16 {
        let span = u64::from(b.saturating_sub(a)) + 1;
        a + (self.next_u64() % span) as u16
    }
}

/// Spectral tilt of a noise source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub enum NoiseColor {
    /// Flat: hiss, crunch.
    White,
    /// −3 dB per octave: rain, rustling.
    Pink,
    /// −6 dB per octave: rumble, wind.
    Brown,
}

/// A noise generator of one colour (pink after Paul Kellet's economy filter).
#[derive(Debug, Clone)]
pub struct Noise {
    color: NoiseColor,
    state: [f32; 3],
}

impl Noise {
    pub fn new(color: NoiseColor) -> Self {
        Self {
            color,
            state: [0.0; 3],
        }
    }

    pub fn next(&mut self, rng: &mut Rng) -> f32 {
        let white = rng.signed();
        let s = &mut self.state;
        match self.color {
            NoiseColor::White => white,
            NoiseColor::Pink => {
                s[0] = 0.99765 * s[0] + white * 0.099_046;
                s[1] = 0.96300 * s[1] + white * 0.296_516_4;
                s[2] = 0.57000 * s[2] + white * 1.052_691_3;
                (s[0] + s[1] + s[2] + white * 0.1848) * 0.25
            }
            NoiseColor::Brown => {
                s[0] = (s[0] + white * 0.02) / 1.02;
                s[0] * 3.5
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    Low,
    High,
    Band,
}

/// Highest frequency a filter or an oscillator may use (below Nyquist).
pub const MAX_HZ: f32 = SAMPLE_RATE as f32 * 0.45;
pub const MIN_HZ: f32 = 20.0;

/// Second-order filter (RBJ cookbook), retunable while running.
#[derive(Debug, Clone)]
pub struct Biquad {
    kind: FilterKind,
    q: f32,
    b: [f32; 3],
    a: [f32; 2],
    x: [f32; 2],
    y: [f32; 2],
}

impl Biquad {
    pub fn new(kind: FilterKind, hz: f32, q: f32) -> Self {
        let mut filter = Self {
            kind,
            q: q.clamp(0.1, 30.0),
            b: [1.0, 0.0, 0.0],
            a: [0.0; 2],
            x: [0.0; 2],
            y: [0.0; 2],
        };
        filter.tune(hz);
        filter
    }

    pub fn tune(&mut self, hz: f32) {
        let w = TAU * hz.clamp(MIN_HZ, MAX_HZ) / SAMPLE_RATE as f32;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * self.q);
        let b = match self.kind {
            FilterKind::Low => [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0],
            FilterKind::High => [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0],
            FilterKind::Band => [alpha, 0.0, -alpha],
        };
        let a0 = 1.0 + alpha;
        self.b = b.map(|v| v / a0);
        self.a = [-2.0 * cos / a0, (1.0 - alpha) / a0];
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let [b0, b1, b2] = self.b;
        let [a1, a2] = self.a;
        let output = b0 * input + b1 * self.x[0] + b2 * self.x[1] - a1 * self.y[0] - a2 * self.y[1];
        self.x = [input, self.x[0]];
        self.y = [output, self.y[0]];
        output
    }
}

/// −60 dB after `seconds`: the decay of struck and plucked things.
pub fn exp_decay(t: f32, seconds: f32) -> f32 {
    (-6.907_755 * t / seconds.max(1e-4)).exp()
}

/// Seconds to samples.
pub fn samples(seconds: f32) -> usize {
    (seconds.max(0.0) * SAMPLE_RATE as f32) as usize
}
