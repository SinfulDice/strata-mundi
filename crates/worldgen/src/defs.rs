//! Generation content read from packs: `dimensions/`, `biomes/` and `features/`.

use serde::Deserialize;
use strata_core::id::Identifier;

/// `dimensions/<name>.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Dimension", deny_unknown_fields)]
pub struct DimensionDef {
    pub terrain: TerrainDef,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainDef {
    pub sea_level: i32,
    /// Below this height, stone becomes deep stone (with a ragged border).
    pub deep_level: i32,
    pub stone: Identifier,
    pub deep_stone: Identifier,
    pub bedrock: Identifier,
    pub water: Identifier,
    pub ice: Identifier,
    /// Multiplies the size of biomes (1 = default, 2 = twice as large).
    #[serde(default = "one")]
    pub biome_scale: f64,
    #[serde(default = "yes")]
    pub caves: bool,
    /// Ore features spread through the whole dimension.
    #[serde(default)]
    pub ores: Vec<Identifier>,
}

/// Where a biome sits in climate space; the nearest biome wins.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Climate {
    pub temperature: f64,
    pub humidity: f64,
    pub continentalness: f64,
    pub erosion: f64,
}

/// `biomes/<name>.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Biome", deny_unknown_fields)]
pub struct BiomeDef {
    pub climate: Climate,
    pub surface: Identifier,
    pub subsurface: Identifier,
    #[serde(default = "default_subsurface_depth")]
    pub subsurface_depth: u32,
    /// Floor of seas and lakes.
    pub underwater: Identifier,
    /// Above each height, the surface becomes this block and the ground below is stone
    /// (rocky slopes, snowy peaks). Sorted by height.
    #[serde(default)]
    pub peaks: Vec<(i32, Identifier)>,
    /// Water surface freezes.
    #[serde(default)]
    pub frozen: bool,
    /// The player may start the game here (welcoming biomes only).
    #[serde(default)]
    pub spawn: bool,
    #[serde(default)]
    pub features: Vec<Placement>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub feature: Identifier,
    /// Average number of attempts per column (16×16 blocks).
    pub per_column: f64,
}

/// `features/<name>.ron`.
#[derive(Debug, Clone, Deserialize)]
pub enum FeatureDef {
    Tree {
        log: Identifier,
        leaves: Identifier,
        height: (u32, u32),
        crown: Crown,
        on: Vec<Identifier>,
    },
    /// A single stack of one block (cactus).
    Pillar {
        block: Identifier,
        height: (u32, u32),
        on: Vec<Identifier>,
    },
    /// Small ore veins inside stone; `deep_block` replaces deep stone.
    Ore {
        block: Identifier,
        deep_block: Option<Identifier>,
        vein_size: u32,
        per_column: f64,
        min_y: i32,
        max_y: i32,
    },
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Crown {
    /// Rounded oak-like crown.
    Round { radius: u32 },
    /// Layered cone (fir).
    Cone { radius: u32 },
}

fn one() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

fn default_subsurface_depth() -> u32 {
    3
}
