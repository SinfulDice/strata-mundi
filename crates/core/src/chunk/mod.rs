//! Block storage: sections of 16³ blocks with a compressed palette, stacked in columns.

mod biomes;
mod column;
mod packed;
mod section;

pub use biomes::{BiomeId, BiomeMap};
pub use column::Column;
pub use packed::{PackedArray, bits_for};
pub use section::{SECTION_VOLUME, Section};
