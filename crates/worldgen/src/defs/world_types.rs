use crate::spline::Spline;
use serde::Deserialize;
use strata_core::id::Identifier;

/// `world_types/<name>.ron`: changes a world type makes to the dimension's generation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "WorldType", deny_unknown_fields)]
pub struct WorldTypeDef {
    /// Position in the list of the creation screen.
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub relief: ReliefChanges,
    /// One biome everywhere (the player may pick another); required by flat types.
    #[serde(default)]
    pub biome: Option<Identifier>,
    /// Layers instead of the relief.
    #[serde(default)]
    pub flat: Option<FlatDef>,
}

/// Multipliers (1 = unchanged) and curves replacing the dimension's.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReliefChanges {
    #[serde(default = "one")]
    pub peaks: f64,
    #[serde(default = "one")]
    pub hills: f64,
    /// Multiplies the largest overhang.
    #[serde(default = "one")]
    pub overhang: f64,
    #[serde(default = "one")]
    pub continent_scale: f64,
    /// Replaces the dimension's remapping of continentalness (see `Relief`).
    #[serde(default)]
    pub continentalness: Option<Spline>,
    #[serde(default)]
    pub max_height: Option<f64>,
}

impl Default for ReliefChanges {
    fn default() -> Self {
        Self {
            peaks: 1.0,
            hills: 1.0,
            overhang: 1.0,
            continent_scale: 1.0,
            continentalness: None,
            max_height: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatDef {
    /// `(block, thickness)` from the bottom of the world up.
    pub layers: Vec<(Identifier, u16)>,
    #[serde(default)]
    pub caves: bool,
    /// Trees and plants of the biome.
    #[serde(default)]
    pub features: bool,
}

fn one() -> f64 {
    1.0
}
