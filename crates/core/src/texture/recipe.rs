use super::color::{HexColor, Palette, Ramp};
use super::{Rgba, RgbaImage, patterns, sprite};
use crate::id::Identifier;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Size of procedurally generated textures, in pixels.
pub const BASE_SIZE: u32 = 16;
const MAX_OCTAVES: u32 = 6;
const MAX_CLUSTERS: u32 = 64;

/// How a texture is drawn, from a `textures/<name>.ron` file. Colors always come from
/// named ramps of the global palette, so a resource pack can restyle everything at once.
#[derive(Debug, Clone, Deserialize)]
pub enum TextureRecipe {
    /// A whole texture painted with one ramp and one pattern.
    Material { ramp: String, pattern: Pattern },
    /// `base` with a pattern painted on top (transparent pixels keep the base).
    Overlay {
        base: Identifier,
        ramp: String,
        pattern: Pattern,
    },
    /// `base` sprinkled with small ore clusters, lit from the top left.
    Ore {
        base: Identifier,
        ramp: String,
        #[serde(default = "default_clusters")]
        clusters: u32,
    },
    /// Hand-drawn pixel art: one character per pixel, `.` = transparent.
    Sprite {
        colors: BTreeMap<char, ColorRef>,
        rows: Vec<String>,
    },
    /// Another texture with some ramps swapped: one drawing, many materials.
    Recolor {
        base: Identifier,
        swap: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub enum Pattern {
    /// Value noise; `cells` = lattice cells across the texture on x and y.
    Noise { cells: (u32, u32), octaves: u32 },
    /// Tree rings with a bark border (log ends).
    Rings { border: String },
    /// Horizontal boards with dark seams (planks).
    Planks { boards: u32 },
    /// Foliage with see-through holes.
    Leaves { holes: f32 },
    /// Vertical ribs (cactus).
    Stripes { period: u32 },
    /// Jagged band hanging from the top edge (grass on the side of dirt).
    TopFringe { depth: u32 },
    /// Border only, transparent center with glints (glass).
    Frame,
}

/// A sprite color: `("ramp", level)` or a fixed `"#rrggbb"`.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ColorRef {
    Ramp(String, usize),
    Hex(HexColor),
}

fn default_clusters() -> u32 {
    4
}

/// What a recipe needs to draw itself.
pub struct Painter<'a> {
    pub palette: &'a Palette,
    pub swap: &'a BTreeMap<String, String>,
    pub seed: u64,
}

impl Painter<'_> {
    pub fn ramp(&self, name: &str) -> Result<&Ramp, String> {
        let name = self.swap.get(name).map_or(name, String::as_str);
        self.palette
            .ramp(name)
            .ok_or_else(|| format!("unknown palette ramp `{name}`"))
    }

    pub fn color(&self, color: &ColorRef) -> Result<Rgba, String> {
        match color {
            ColorRef::Hex(hex) => Ok(hex.0),
            ColorRef::Ramp(name, level) => {
                let ramp = self.ramp(name)?;
                ramp.get(*level)
                    .copied()
                    .ok_or_else(|| format!("ramp `{name}` has no level {level}"))
            }
        }
    }
}

impl TextureRecipe {
    /// Texture this recipe is painted on top of.
    pub fn base(&self) -> Option<&Identifier> {
        match self {
            Self::Overlay { base, .. } | Self::Ore { base, .. } => Some(base),
            Self::Material { .. } | Self::Sprite { .. } | Self::Recolor { .. } => None,
        }
    }

    /// Paints this recipe's own pixels on a transparent `BASE_SIZE` canvas; the caller
    /// composes it over the base texture, whatever the base resolution.
    pub fn paint_layer(&self, painter: &Painter<'_>) -> Result<RgbaImage, String> {
        self.validate()?;
        let blank = RgbaImage::new(BASE_SIZE, BASE_SIZE, Rgba([0; 4]));
        match self {
            Self::Material { ramp, pattern } | Self::Overlay { ramp, pattern, .. } => {
                patterns::paint(blank, painter.ramp(ramp)?, pattern, painter)
            }
            Self::Ore { ramp, clusters, .. } => Ok(patterns::ore(
                blank,
                painter.ramp(ramp)?,
                *clusters,
                painter.seed,
            )),
            Self::Sprite { colors, rows } => sprite::draw(colors, rows, painter),
            Self::Recolor { .. } => Err("recolors are resolved by `generate`".into()),
        }
    }

    /// Pack data must never freeze or crash the game: parameters are bounded.
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Ore { clusters, .. } => at_most("clusters", *clusters, MAX_CLUSTERS),
            Self::Material { pattern, .. } | Self::Overlay { pattern, .. } => match pattern {
                Pattern::Noise { cells, octaves } => {
                    at_most("octaves", *octaves, MAX_OCTAVES)?;
                    at_most("cells", cells.0.max(cells.1), BASE_SIZE)
                }
                Pattern::Planks { boards } => at_most("boards", *boards, BASE_SIZE),
                Pattern::Stripes { period } => at_most("period", *period, BASE_SIZE),
                Pattern::TopFringe { depth } => at_most("depth", *depth, BASE_SIZE),
                Pattern::Leaves { holes } if !(0.0..=1.0).contains(holes) => {
                    Err("`holes` must be between 0 and 1".into())
                }
                Pattern::Leaves { .. } | Pattern::Rings { .. } | Pattern::Frame => Ok(()),
            },
            Self::Sprite { .. } | Self::Recolor { .. } => Ok(()),
        }
    }
}

fn at_most(name: &str, value: u32, max: u32) -> Result<(), String> {
    if value <= max {
        Ok(())
    } else {
        Err(format!("`{name}` is {value}, the maximum is {max}"))
    }
}
