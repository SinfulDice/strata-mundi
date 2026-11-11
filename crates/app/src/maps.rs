//! The generation as the creation screen sees it: the world types of the packs and maps
//! of what settings generate.

use std::sync::Arc;
use strata_client::generation::{
    GenerationCatalog, GenerationError, MapRequest, MapSource, WorldTypeChoice,
};
use strata_core::generation::WorldgenSettings;
use strata_worldgen::map::{self, MapArea};
use strata_worldgen::{Generator, Seed, SettingsError, WorldgenContent};

pub fn catalog(worldgen: &WorldgenContent) -> GenerationCatalog {
    GenerationCatalog {
        types: worldgen
            .world_types()
            .iter()
            .map(|t| WorldTypeChoice {
                id: t.id.clone(),
                biome: t.biome.clone(),
                flat: t.flat.clone(),
            })
            .collect(),
        biomes: worldgen.biomes().iter().map(|b| b.id.clone()).collect(),
        sea_level: worldgen.terrain.sea_level,
    }
}

/// Maps centered on the origin, drawn on `threads` threads.
pub struct GeneratorMaps {
    pub worldgen: Arc<WorldgenContent>,
    pub threads: usize,
}

impl MapSource for GeneratorMaps {
    fn render(
        &self,
        request: &MapRequest,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Vec<[u8; 3]>>, GenerationError> {
        let seed = Seed::from_text(&request.seed);
        let generator =
            Generator::with_settings(seed, &self.worldgen, &request.settings).map_err(explain)?;
        let area = MapArea {
            center: (0, 0),
            size: request.size,
            scale: request.scale,
        };
        Ok(map::render_with(&generator, area, self.threads, cancelled))
    }
}

/// The settings as saved, if the packs can make a world with them (said to the player if
/// not).
pub fn check(
    worldgen: &WorldgenContent,
    settings: &WorldgenSettings,
) -> Result<WorldgenSettings, GenerationError> {
    let used = worldgen.used_settings(settings).map_err(explain)?;
    worldgen.with_settings(&used).map_err(explain)?;
    Ok(used)
}

fn explain(error: SettingsError) -> GenerationError {
    GenerationError {
        message: error.message_id(),
        id: error.identifier().map(ToString::to_string),
    }
}
