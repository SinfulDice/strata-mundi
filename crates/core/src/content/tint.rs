//! Biome colours: grass, foliage and water textures multiplied by a factor of the biome.

use serde::{Deserialize, Serialize};

/// Which colour of the biome a block takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TintKind {
    Grass,
    Foliage,
    Water,
}

impl TintKind {
    pub const ALL: [TintKind; 3] = [Self::Grass, Self::Foliage, Self::Water];

    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Faces of a block taking the biome colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum TintFaces {
    /// Every face, every pixel.
    #[default]
    All,
    /// The top, and the pixels of the sides marked as tinted (grass fringe): the side
    /// texture's pixels with alpha 254 keep their colour. The bottom keeps its colour.
    TopFringe,
}

/// `tint` field of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BlockTint {
    pub kind: TintKind,
    #[serde(default)]
    pub faces: TintFaces,
}

/// Alpha marking the pixels of a `TopFringe` side that keep their colour.
pub const UNTINTED_ALPHA: u8 = 254;

/// Colour factor per channel, in display space: 128 = ×1 (the texture as painted), 255
/// almost ×2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tint(pub [u8; 3]);

impl Tint {
    pub const NEUTRAL: Tint = Tint([128; 3]);

    /// Factors from a pack (1.0 = unchanged), bounded to `0..2`.
    pub fn from_factors(factors: [f32; 3]) -> Self {
        Self(factors.map(|f| (f.clamp(0.0, 2.0) * 128.0).round().min(255.0) as u8))
    }

    pub fn factors(self) -> [f32; 3] {
        self.0.map(|c| f32::from(c) / 128.0)
    }
}

impl Default for Tint {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

/// The colours of a biome, as the server sends them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BiomeTints {
    /// By `TintKind::index`.
    pub kinds: [Tint; 3],
    /// Multiplies the colour of the fog (the sky's horizon).
    pub fog: Tint,
}

impl BiomeTints {
    pub fn get(&self, kind: TintKind) -> Tint {
        self.kinds[kind.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factors_round_trip_and_are_bounded() {
        assert_eq!(Tint::from_factors([1.0; 3]), Tint::NEUTRAL);
        assert_eq!(Tint::from_factors([1.0; 3]).factors(), [1.0; 3]);
        assert_eq!(Tint::from_factors([-1.0, 0.5, 9.0]).0, [0, 64, 255]);
    }
}
