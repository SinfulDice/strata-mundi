//! Generation content read from packs: `dimensions/`, `biomes/` and `features/`.

mod features;

pub use features::{Crown, FeatureDef, Trunk};

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
    /// `None` = no caves at all.
    #[serde(default)]
    pub caves: Option<CavesDef>,
    /// Ore features spread through the whole dimension.
    #[serde(default)]
    pub ores: Vec<Identifier>,
    /// Giant ore veins, rare and rich (`Deposit` features).
    #[serde(default)]
    pub deposits: Vec<Identifier>,
    pub relief: Relief,
}

/// Caves carved after the terrain. Widths are shares of the noise range around zero:
/// a wider share gives wider tunnels.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CavesDef {
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Multiplies every kind of cave at once (0 = none, 1 = default).
    #[serde(default = "one")]
    pub density: f64,
    #[serde(default = "default_spaghetti")]
    pub spaghetti: f64,
    #[serde(default = "default_noodles")]
    pub noodles: f64,
    /// Noodles stay below this height.
    #[serde(default = "default_noodle_top")]
    pub noodle_top: i32,
    /// Cheese noise above which caverns open, deep down; higher = fewer caverns.
    #[serde(default = "default_cheese")]
    pub cheese: f64,
    /// Caverns thin out towards this height and stop a third higher.
    #[serde(default = "default_cavern_top")]
    pub cavern_top: i32,
    #[serde(default = "default_ravines")]
    pub ravines: f64,
    /// Smallest and largest depth of ravines, in blocks.
    #[serde(default = "default_ravine_depth")]
    pub ravine_depth: (i32, i32),
    /// Share of the regions whose caves are flooded below their own level.
    #[serde(default = "default_aquifers")]
    pub aquifers: f64,
    /// Side of an aquifer region, in blocks.
    #[serde(default = "default_aquifer_cell")]
    pub aquifer_cell: i32,
    /// Highest water level of aquifers.
    #[serde(default = "default_aquifer_top")]
    pub aquifer_top: i32,
    /// Aquifer levels range from `aquifer_top - aquifer_depth` to `aquifer_top`.
    #[serde(default = "default_aquifer_depth")]
    pub aquifer_depth: i32,
    /// Caves at or below this height fill with lava.
    #[serde(default = "default_lava_level")]
    pub lava_level: i32,
    pub lava: Identifier,
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
    #[serde(default)]
    pub wetlands: Option<Wetlands>,
}

/// Humid, worn-down lowlands sink to the sea level: shallow pools and islets (marshes).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wetlands {
    /// Humidity → 0 to 1.
    pub humidity: Spline,
    /// Erosion → 0 to 1.
    pub erosion: Spline,
    /// Height of the flattened land, in blocks above the sea (negative = under water).
    pub level: f64,
    /// Bumps of the flattened land, in blocks.
    pub bumps: f64,
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
    /// Highest ground, above the sea level, still covered with `underwater` (shores).
    #[serde(default = "default_shore_height")]
    pub shore_height: i32,
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
    #[serde(default)]
    pub bands: Option<BandsDef>,
    /// Color on the generation preview map.
    #[serde(default = "grey")]
    pub map_color: HexColor,
    /// The player may start the game here (welcoming biomes only).
    #[serde(default)]
    pub spawn: bool,
    #[serde(default)]
    pub features: Vec<Placement>,
}

/// Coloured layers by height under the surface and on cliffs (badlands strata).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandsDef {
    /// One block per height, repeated upwards; repeat a block for a thicker layer.
    pub blocks: Vec<Identifier>,
    /// How far below the surface the layers go, in blocks.
    pub depth: u32,
    /// Waviness of the layers, in blocks.
    #[serde(default = "default_band_waviness")]
    pub waviness: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub feature: Identifier,
    /// Average number of attempts per column (16×16 blocks); for plants, patches per
    /// column, at most 8.
    pub per_column: f64,
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

fn default_spaghetti() -> f64 {
    0.075
}

fn default_noodles() -> f64 {
    0.05
}

fn default_noodle_top() -> i32 {
    100
}

fn default_cheese() -> f64 {
    0.3
}

fn default_cavern_top() -> i32 {
    140
}

fn default_ravine_depth() -> (i32, i32) {
    (30, 80)
}

fn default_aquifer_cell() -> i32 {
    64
}

fn default_aquifer_depth() -> i32 {
    48
}

fn default_ravines() -> f64 {
    0.03
}

fn default_aquifers() -> f64 {
    0.35
}

fn default_aquifer_top() -> i32 {
    100
}

fn default_lava_level() -> i32 {
    10
}

fn yes() -> bool {
    true
}

fn default_subsurface_depth() -> u32 {
    3
}

fn default_shore_height() -> i32 {
    1
}

fn default_band_waviness() -> f64 {
    2.0
}
