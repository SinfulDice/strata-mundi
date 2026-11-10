//! Sounds made by code: recipes from the packs rendered into mono buffers. Pure and
//! deterministic for a seed (no audio device), so everything here is tested anywhere.

mod dsp;
mod load;
mod recipe;
mod synth;
#[cfg(test)]
mod tests;
mod wav;

pub use dsp::{NoiseColor, Rng};
pub use load::{Sound, SoundSet, SoundSetDef, Sounds, load_sounds, variant_seed};
pub use recipe::{
    Band, Envelope, Filter, Layer, MAX_GRAINS, MAX_LAYERS, Mode, SoundCategory, SoundDef, Wobble,
};
pub use synth::render;
pub use wav::{wav_bytes, write_wav};

/// Samples per second of every rendered buffer.
pub const SAMPLE_RATE: u32 = 44_100;
/// Longest buffer a recipe may ask for.
pub const MAX_SECONDS: f32 = 10.0;
/// Most variants a sound may have.
pub const MAX_VARIANTS: u8 = 16;
