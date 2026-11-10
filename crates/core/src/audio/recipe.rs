//! Sound recipes as written in `sounds/<name>.ron`, and the limits they are held to.
//! Pairs `(a, b)` are ranges: each rendered variant picks its own value in between.

use super::dsp::{MAX_HZ, MIN_HZ, NoiseColor};
use super::{MAX_SECONDS, MAX_VARIANTS};
use serde::Deserialize;

/// Volume family of a sound (one slider each in the options, under the master).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum SoundCategory {
    Blocks,
    Ambient,
    Music,
}

impl SoundCategory {
    pub const ALL: [SoundCategory; 3] = [Self::Blocks, Self::Ambient, Self::Music];
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Sound", deny_unknown_fields)]
pub struct SoundDef {
    pub category: SoundCategory,
    /// Fluent key of the subtitle; none: never subtitled (music).
    #[serde(default)]
    pub subtitle: Option<String>,
    /// Peak of the rendered buffer, 0 to 1.
    #[serde(default = "one")]
    pub volume: f32,
    /// Playback speed picked at each play (higher and shorter above 1).
    #[serde(default = "unit_pitch")]
    pub pitch: (f32, f32),
    /// Buffers rendered with different seeds; each play picks one.
    #[serde(default = "default_variants")]
    pub variants: u8,
    /// Length of the buffer in seconds; what rings longer is faded out.
    pub duration: f32,
    /// Blocks beyond which it is not heard.
    #[serde(default = "default_range")]
    pub range: f32,
    /// Played over and over (ambience beds): the end is blended into the start.
    #[serde(default)]
    pub looped: bool,
    pub layers: Vec<Layer>,
}

/// One ingredient of a recipe; the layers are summed, then the whole is normalised.
#[derive(Debug, Clone, Deserialize)]
pub enum Layer {
    /// A shaped burst of filtered noise (scuffs, hiss, splashes, wind).
    Noise {
        color: NoiseColor,
        #[serde(default)]
        filter: Filter,
        envelope: Envelope,
        #[serde(default = "one")]
        gain: f32,
        #[serde(default)]
        delay: (f32, f32),
        /// Slow random swell of loudness and filter (wind gusts).
        #[serde(default)]
        wobble: Option<Wobble>,
    },
    /// Many tiny noise bursts scattered in time (gravel crunch, snow, leaves, crackling).
    Grains {
        count: (u16, u16),
        /// Seconds over which the grains start.
        spread: f32,
        /// Seconds each grain rings.
        length: (f32, f32),
        color: NoiseColor,
        #[serde(default)]
        filter: Filter,
        #[serde(default = "one")]
        gain: f32,
        /// Loudness of the last grains relative to the first.
        #[serde(default = "one")]
        fade: f32,
        #[serde(default)]
        delay: (f32, f32),
    },
    /// Decaying sine partials: the body of struck wood, stone, glass or metal.
    Modes {
        modes: Vec<Mode>,
        #[serde(default = "one")]
        gain: f32,
        #[serde(default)]
        delay: (f32, f32),
    },
    /// Rising sine chirps (air bubbles in water or lava).
    Bubbles {
        count: (u16, u16),
        spread: f32,
        hz: (f32, f32),
        /// Pitch reached at the end of a bubble, as a ratio of its start.
        #[serde(default = "rise")]
        rise: f32,
        decay: (f32, f32),
        #[serde(default = "one")]
        gain: f32,
        #[serde(default)]
        delay: (f32, f32),
    },
}

#[derive(Debug, Clone, Default, Deserialize)]
pub enum Filter {
    #[default]
    None,
    LowPass(Band),
    HighPass(Band),
    BandPass(Band),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    pub hz: (f32, f32),
    #[serde(default = "default_q")]
    pub q: f32,
    /// Ratio of the frequency at the end of the layer to its start.
    #[serde(default = "one")]
    pub sweep: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    #[serde(default = "default_attack")]
    pub attack: f32,
    #[serde(default)]
    pub hold: (f32, f32),
    /// Seconds to fall by 60 dB.
    pub decay: (f32, f32),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mode {
    pub hz: (f32, f32),
    pub decay: (f32, f32),
    #[serde(default = "one")]
    pub gain: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wobble {
    /// Swells per second.
    pub rate: f32,
    /// 0 (steady) to 1 (fades to silence between gusts).
    pub depth: f32,
}

/// Most grains or bubbles a layer may ask for.
pub const MAX_GRAINS: u16 = 512;
/// Most layers in a recipe and partials in a `Modes` layer.
pub const MAX_LAYERS: usize = 16;

fn one() -> f32 {
    1.0
}

fn unit_pitch() -> (f32, f32) {
    (1.0, 1.0)
}

fn default_variants() -> u8 {
    4
}

fn default_range() -> f32 {
    16.0
}

fn default_q() -> f32 {
    std::f32::consts::FRAC_1_SQRT_2
}

fn default_attack() -> f32 {
    0.002
}

fn rise() -> f32 {
    1.5
}

/// Holds the values of a recipe inside what the synthesiser handles; each correction
/// is described in `problems` (reported as pack errors).
pub(super) fn bound(def: &mut SoundDef, problems: &mut Vec<String>) {
    let mut b = Bounds { problems };
    b.value("duration", &mut def.duration, 0.01, MAX_SECONDS);
    b.value("volume", &mut def.volume, 0.0, 1.0);
    b.value("range", &mut def.range, 1.0, 256.0);
    b.pair("pitch", &mut def.pitch, 0.25, 4.0);
    if def.variants == 0 || def.variants > MAX_VARIANTS {
        b.problem(format!("`variants` must be between 1 and {MAX_VARIANTS}"));
        def.variants = def.variants.clamp(1, MAX_VARIANTS);
    }
    if def.layers.is_empty() {
        b.problem("a sound needs at least one layer".to_owned());
    }
    if def.layers.len() > MAX_LAYERS {
        b.problem(format!("at most {MAX_LAYERS} layers"));
        def.layers.truncate(MAX_LAYERS);
    }
    for layer in &mut def.layers {
        b.layer(layer);
    }
}

struct Bounds<'a> {
    problems: &'a mut Vec<String>,
}

impl Bounds<'_> {
    fn problem(&mut self, problem: String) {
        self.problems.push(problem);
    }

    fn value(&mut self, name: &str, value: &mut f32, min: f32, max: f32) {
        if value.is_finite() && (min..=max).contains(value) {
            return;
        }
        self.problem(format!("`{name}` must be between {min} and {max}"));
        *value = if value.is_finite() {
            value.clamp(min, max)
        } else {
            min
        };
    }

    fn pair(&mut self, name: &str, (a, b): &mut (f32, f32), min: f32, max: f32) {
        self.value(name, a, min, max);
        self.value(name, b, min, max);
    }

    fn count(&mut self, count: &mut (u16, u16)) {
        if count.0 > MAX_GRAINS || count.1 > MAX_GRAINS {
            self.problem(format!("`count` must be at most {MAX_GRAINS}"));
        }
        count.1 = count.1.min(MAX_GRAINS);
        count.0 = count.0.min(count.1);
    }

    fn filter(&mut self, filter: &mut Filter) {
        if let Filter::LowPass(band) | Filter::HighPass(band) | Filter::BandPass(band) = filter {
            self.pair("hz", &mut band.hz, MIN_HZ, MAX_HZ);
            self.value("q", &mut band.q, 0.1, 30.0);
            self.value("sweep", &mut band.sweep, 0.05, 20.0);
        }
    }

    fn layer(&mut self, layer: &mut Layer) {
        match layer {
            Layer::Noise {
                filter,
                envelope,
                gain,
                delay,
                wobble,
                ..
            } => {
                self.filter(filter);
                self.value("attack", &mut envelope.attack, 0.0, MAX_SECONDS);
                self.pair("hold", &mut envelope.hold, 0.0, MAX_SECONDS);
                self.pair("decay", &mut envelope.decay, 0.001, MAX_SECONDS);
                self.value("gain", gain, 0.0, 4.0);
                self.pair("delay", delay, 0.0, MAX_SECONDS);
                if let Some(wobble) = wobble {
                    self.value("rate", &mut wobble.rate, 0.0, 20.0);
                    self.value("depth", &mut wobble.depth, 0.0, 1.0);
                }
            }
            Layer::Grains {
                count,
                spread,
                length,
                filter,
                gain,
                fade,
                delay,
                ..
            } => {
                self.count(count);
                self.value("spread", spread, 0.0, MAX_SECONDS);
                self.pair("length", length, 0.0005, 1.0);
                self.filter(filter);
                self.value("gain", gain, 0.0, 4.0);
                self.value("fade", fade, 0.0, 4.0);
                self.pair("delay", delay, 0.0, MAX_SECONDS);
            }
            Layer::Modes { modes, gain, delay } => {
                if modes.len() > MAX_LAYERS {
                    self.problem(format!("at most {MAX_LAYERS} modes"));
                    modes.truncate(MAX_LAYERS);
                }
                for mode in modes {
                    self.pair("hz", &mut mode.hz, MIN_HZ, MAX_HZ);
                    self.pair("decay", &mut mode.decay, 0.001, MAX_SECONDS);
                    self.value("gain", &mut mode.gain, 0.0, 4.0);
                }
                self.value("gain", gain, 0.0, 4.0);
                self.pair("delay", delay, 0.0, MAX_SECONDS);
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
                self.count(count);
                self.value("spread", spread, 0.0, MAX_SECONDS);
                self.pair("hz", hz, MIN_HZ, MAX_HZ / 4.0);
                self.value("rise", rise, 0.25, 4.0);
                self.pair("decay", decay, 0.001, 2.0);
                self.value("gain", gain, 0.0, 4.0);
                self.pair("delay", delay, 0.0, MAX_SECONDS);
            }
        }
    }
}
