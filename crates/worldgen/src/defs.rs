//! Generation content read from packs: `dimensions/`, `biomes/` and `features/`.

use crate::spline::Spline;
use serde::Deserialize;
use strata_core::id::Identifier;
use strata_core::texture::{HexColor, Rgba};

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
    pub relief: Relief,
}

/// Shape of the land, as curves of the climate maps (like Minecraft's Caves & Cliffs).
/// Heights are in blocks above the sea; erosion and peaks-and-valleys are in `[-1, 1]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relief {
    /// Continentalness → base height: deep oceans, coasts, inland.
    pub continent: Spline,
    /// Continentalness → 0 at sea to 1 inland: how much hills and mountains rise.
    pub inland: Spline,
    /// Erosion → 0 (worn-down hills) to 1 (mountain ranges).
    pub rugged: Spline,
    /// Peaks-and-valleys → height of mountains.
    pub peaks: Spline,
    /// Peaks-and-valleys → height of hills where the land is worn down.
    pub hills: Spline,
    /// Erosion → 0 to 1: raised, terraced plateaus.
    #[serde(default = "zero")]
    pub plateau: Spline,
    #[serde(default)]
    pub plateau_lift: f64,
    /// Height of plateau steps, in blocks.
    #[serde(default = "default_terrace")]
    pub terrace_step: f64,
    /// Erosion → 0 to 1: how much 3D noise carves cliffs, overhangs and arches.
    pub overhang: Spline,
    /// Largest overhang, in blocks.
    pub overhang_blocks: f64,
    /// Size of the overhang noise's shapes, in blocks.
    #[serde(default = "default_overhang_scale")]
    pub overhang_scale: f64,
    /// Size of mountain ranges (peaks-and-valleys), in blocks.
    #[serde(default = "default_weirdness_scale")]
    pub weirdness_scale: f64,
    /// Height difference to a neighbour from which the ground is bare rock.
    #[serde(default = "default_steep_slope")]
    pub steep_slope: i32,
    #[serde(default)]
    pub rivers: Option<Rivers>,
    /// Distance between rivers, roughly, in blocks.
    #[serde(default = "default_river_scale")]
    pub river_scale: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rivers {
    /// Share of the river noise that is water (around 0.02).
    pub width: f64,
    /// Share of the river noise that slopes down to the banks.
    pub banks: f64,
    /// Depth of the bed below the sea level.
    pub depth: f64,
    /// Rivers fade out above this height (blocks above the sea), carving dry valleys.
    pub highest: f64,
    /// River factor (0 to 1) from which the river biome is chosen.
    #[serde(default = "default_river_biome")]
    pub biome: f64,
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
    /// Surface of steep slopes (stone by default).
    #[serde(default)]
    pub steep: Option<Identifier>,
    /// Water surface freezes.
    #[serde(default)]
    pub frozen: bool,
    /// Only chosen where the plain terrain height is in this range.
    #[serde(default)]
    pub min_height: Option<i32>,
    #[serde(default)]
    pub max_height: Option<i32>,
    /// Only chosen on river beds (and never elsewhere).
    #[serde(default)]
    pub river: bool,
    /// Color on the generation preview map.
    #[serde(default = "grey")]
    pub map_color: HexColor,
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

fn zero() -> Spline {
    Spline::constant(0.0)
}

fn grey() -> HexColor {
    HexColor(Rgba([128, 128, 128, 255]))
}

fn default_overhang_scale() -> f64 {
    44.0
}

fn default_weirdness_scale() -> f64 {
    650.0
}

fn default_steep_slope() -> i32 {
    3
}

fn default_river_scale() -> f64 {
    380.0
}

fn default_river_biome() -> f64 {
    0.6
}

fn default_terrace() -> f64 {
    8.0
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
