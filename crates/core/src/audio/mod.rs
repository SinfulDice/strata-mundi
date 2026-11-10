//! Sounds made by code: recipes from the packs rendered into mono buffers. Pure and
//! deterministic for a seed (no audio device), so everything here is tested anywhere.

mod ambience;
mod dsp;
mod load;
pub mod music;
mod recipe;
mod spaces;
mod synth;
#[cfg(test)]
mod tests;
mod wav;

pub use ambience::{
    Altitude, Ambience, AmbienceDef, Bed, BedDef, Event, EventDef, MAX_PARTS, TimeOfDay,
};
pub use dsp::{NoiseColor, Rng};
pub use load::{Sound, SoundSet, SoundSetDef, Sounds, load_sounds, variant_seed};
pub use recipe::{
    Band, Envelope, Filter, Layer, MAX_GRAINS, MAX_LAYERS, Mode, SoundCategory, SoundDef, Wobble,
};
pub use spaces::{MAX_FEEDBACK, SpaceDef};
pub use synth::{render, seamless};
pub use wav::{WAV_HEADER, wav_bytes, wav_header, write_wav};

/// Samples per second of every rendered buffer.
pub const SAMPLE_RATE: u32 = 44_100;
/// Longest buffer a recipe may ask for.
pub const MAX_SECONDS: f32 = 10.0;
/// Most variants a sound may have.
pub const MAX_VARIANTS: u8 = 16;
