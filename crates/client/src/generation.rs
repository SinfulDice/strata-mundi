//! What the world creation screen needs to know about generation. The program around the
//! client owns the generator: it lists the world types of the packs and draws maps.

use strata_core::generation::{FlatSettings, WorldgenSettings};
use strata_core::id::Identifier;

/// A world type of the packs, with what the editor starts from.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldTypeChoice {
    pub id: Identifier,
    /// The biome forced by default (single biome and flat types).
    pub biome: Option<Identifier>,
    /// Default layers of flat types.
    pub flat: Option<FlatSettings>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GenerationCatalog {
    /// In the order of the creation screen; the first is the default.
    pub types: Vec<WorldTypeChoice>,
    pub biomes: Vec<Identifier>,
    /// The dimension's sea level (the `None` of the settings).
    pub sea_level: i32,
}

impl GenerationCatalog {
    pub fn world_type(&self, id: &Identifier) -> Option<&WorldTypeChoice> {
        self.types.iter().find(|t| &t.id == id)
    }
}

/// A map of what `settings` generate around the origin.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRequest {
    pub seed: String,
    pub settings: WorldgenSettings,
    /// In pixels.
    pub size: (u32, u32),
    /// Blocks per pixel.
    pub scale: u32,
}

/// Why settings cannot make a world (unknown type, biome or block, bad code…): a Fluent
/// message and its `id` argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationError {
    pub message: &'static str,
    pub id: Option<String>,
}

impl GenerationError {
    pub fn new(message: &'static str) -> Self {
        Self { message, id: None }
    }

    pub fn about(message: &'static str, id: &impl ToString) -> Self {
        Self {
            message,
            id: Some(id.to_string()),
        }
    }
}

pub trait MapSource: Send + Sync {
    /// RGB pixels row by row from the north-west corner, `Ok(None)` once `cancelled`.
    fn render(
        &self,
        request: &MapRequest,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Vec<[u8; 3]>>, GenerationError>;
}
