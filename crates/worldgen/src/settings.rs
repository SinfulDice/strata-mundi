//! The player's generation settings applied over the pack: world type, sea level, sizes
//! and densities. Only generic mechanisms here; the types themselves live in `world_types/`.

use crate::content::bounded_relief;
use crate::content::{Feature, Flat, WorldType, WorldgenContent};
use crate::defs::{Relief, ReliefChanges};
use strata_core::generation::{FlatSettings, WorldgenSettings};
use strata_core::id::Identifier;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SettingsError {
    #[error("unknown world type `{0}`")]
    UnknownType(Identifier),
    #[error("unknown biome `{0}`")]
    UnknownBiome(Identifier),
    #[error("unknown block `{0}`")]
    UnknownBlock(Identifier),
    #[error("a flat world needs at least one layer")]
    NoLayers,
}

impl SettingsError {
    /// Fluent message explaining the error to the player (argument `id`).
    pub fn message_id(&self) -> &'static str {
        match self {
            Self::UnknownType(_) => "world-settings-unknown-type",
            Self::UnknownBiome(_) => "world-settings-unknown-biome",
            Self::UnknownBlock(_) => "world-settings-unknown-block",
            Self::NoLayers => "world-settings-no-layers",
        }
    }

    pub fn identifier(&self) -> Option<&Identifier> {
        match self {
            Self::UnknownType(id) | Self::UnknownBiome(id) | Self::UnknownBlock(id) => Some(id),
            Self::NoLayers => None,
        }
    }
}

impl WorldgenContent {
    /// The generation content changed by the settings (bounded first).
    pub fn with_settings(&self, settings: &WorldgenSettings) -> Result<Self, SettingsError> {
        let settings = self.used_settings(settings)?;
        let world_type = self.world_type(&settings.world_type)?;
        let mut content = self.clone();
        let forced = world_type
            .biome
            .as_ref()
            .map(|default| self.biome_index(settings.biome.as_ref().unwrap_or(default)))
            .transpose()?;
        let flat = world_type
            .flat
            .as_ref()
            .map(|default| self.flat(settings.flat.as_ref().unwrap_or(default)))
            .transpose()?;
        let terrain = &mut content.terrain;
        if let Some(level) = settings.sea_level {
            terrain.sea_level = level;
        }
        let factor = WorldgenSettings::factor;
        terrain.biome_scale = (terrain.biome_scale * factor(settings.biome_size)).clamp(0.25, 16.0);
        terrain.relief = changed_relief(&terrain.relief, &world_type.relief);
        terrain.forced_biome = forced;
        let mut caves = factor(settings.caves);
        if let Some((flat, settings)) = flat {
            // No sea over the layers: everything is dry land.
            terrain.sea_level = 0;
            caves *= if settings.caves { 1.0 } else { 0.0 };
            terrain.flat = Some(flat);
        }
        terrain.scale_caves(caves);
        content.scale_ores(factor(settings.ores));
        Ok(content)
    }

    /// `settings` bounded, without the biome or layers its type does not use (as saved).
    pub fn used_settings(
        &self,
        settings: &WorldgenSettings,
    ) -> Result<WorldgenSettings, SettingsError> {
        let mut settings = settings.clone().bounded();
        let world_type = self.world_type(&settings.world_type)?;
        if world_type.biome.is_none() {
            settings.biome = None;
        }
        if world_type.flat.is_none() {
            settings.flat = None;
        }
        Ok(settings)
    }

    fn world_type(&self, id: &Identifier) -> Result<&WorldType, SettingsError> {
        self.world_types()
            .iter()
            .find(|t| &t.id == id)
            .ok_or_else(|| SettingsError::UnknownType(id.clone()))
    }

    fn biome_index(&self, id: &Identifier) -> Result<usize, SettingsError> {
        self.biomes()
            .iter()
            .position(|biome| &biome.id == id)
            .ok_or_else(|| SettingsError::UnknownBiome(id.clone()))
    }

    fn flat<'a>(
        &self,
        settings: &'a FlatSettings,
    ) -> Result<(Flat, &'a FlatSettings), SettingsError> {
        let layers = settings
            .layers
            .iter()
            .map(|layer| {
                let block = self
                    .blocks
                    .get(&layer.block)
                    .ok_or_else(|| SettingsError::UnknownBlock(layer.block.clone()))?;
                Ok((*block, i32::from(layer.thickness)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if layers.is_empty() {
            return Err(SettingsError::NoLayers);
        }
        let flat = Flat {
            layers,
            features: settings.features,
        };
        Ok((flat, settings))
    }

    fn scale_ores(&mut self, factor: f64) {
        if factor <= 0.0 {
            self.terrain.ores.clear();
            self.terrain.deposits.clear();
            return;
        }
        let indices = self.terrain.ores.iter().chain(&self.terrain.deposits);
        for &index in indices {
            match &mut self.features[index] {
                Feature::Ore { per_column, .. } => *per_column = (*per_column * factor).min(256.0),
                Feature::Deposit(deposit) => deposit.chance = (deposit.chance * factor).min(1.0),
                _ => {}
            }
        }
    }
}

fn changed_relief(relief: &Relief, changes: &ReliefChanges) -> Relief {
    let mut relief = relief.clone();
    relief.peaks = relief.peaks.scaled(changes.peaks);
    relief.hills = relief.hills.scaled(changes.hills);
    relief.overhang_blocks *= changes.overhang;
    relief.continent_scale *= changes.continent_scale;
    if let Some(continentalness) = &changes.continentalness {
        relief.continentalness = Some(continentalness.clone());
    }
    if let Some(max_height) = changes.max_height {
        relief.max_height = max_height;
    }
    bounded_relief(&relief)
}
