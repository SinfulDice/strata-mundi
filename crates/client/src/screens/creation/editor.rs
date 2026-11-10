//! The generation settings being edited: bounded sliders, layers of flat worlds, the one
//! biome, and codes. Values equal to the world type's default are stored as "default".

use crate::generation::{GenerationCatalog, WorldTypeChoice};
use std::ops::RangeInclusive;
use std::sync::Arc;
use strata_core::generation::{
    BIOME_SIZES, DENSITIES, FlatLayer, FlatSettings, MAX_FLAT_LAYERS, SEA_LEVELS, WorldgenSettings,
};
use strata_core::id::Identifier;

pub const MAX_THICKNESS: u16 = 64;
const PERCENT_STEP: i32 = 25;
const SEA_STEP: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slider {
    BiomeSize,
    SeaLevel,
    Caves,
    Ores,
}

impl Slider {
    fn range(self) -> RangeInclusive<i32> {
        let widen = |r: RangeInclusive<u16>| i32::from(*r.start())..=i32::from(*r.end());
        match self {
            Self::BiomeSize => widen(BIOME_SIZES),
            Self::SeaLevel => SEA_LEVELS,
            Self::Caves | Self::Ores => widen(DENSITIES),
        }
    }

    fn step(self) -> i32 {
        match self {
            Self::SeaLevel => SEA_STEP,
            _ => PERCENT_STEP,
        }
    }
}

/// Why a code was refused: a Fluent message and its `id` argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub message: &'static str,
    pub id: Option<String>,
}

impl Refused {
    fn unknown(message: &'static str, id: &Identifier) -> Self {
        Self {
            message,
            id: Some(id.to_string()),
        }
    }
}

pub struct GenerationEditor {
    pub settings: WorldgenSettings,
    catalog: Arc<GenerationCatalog>,
    /// Blocks layers can be made of, in the order offered.
    blocks: Vec<Identifier>,
    /// Selected layer, counted from the bottom.
    pub layer: usize,
}

impl GenerationEditor {
    pub fn new(catalog: Arc<GenerationCatalog>, blocks: Vec<Identifier>) -> Self {
        let world_type = catalog
            .types
            .first()
            .map_or_else(WorldgenSettings::normal, |t| t.id.clone());
        Self {
            settings: WorldgenSettings {
                world_type,
                ..WorldgenSettings::default()
            },
            catalog,
            blocks,
            layer: 0,
        }
    }

    pub fn catalog(&self) -> &GenerationCatalog {
        &self.catalog
    }

    pub fn blocks(&self) -> &[Identifier] {
        &self.blocks
    }

    pub fn world_type(&self) -> Option<&WorldTypeChoice> {
        self.catalog.world_type(&self.settings.world_type)
    }

    /// Next (`step` > 0) or previous type; its own biome and layers come back.
    pub fn cycle_type(&mut self, step: i32) {
        let types = &self.catalog.types;
        if types.is_empty() {
            return;
        }
        let current = types
            .iter()
            .position(|t| t.id == self.settings.world_type)
            .unwrap_or(0) as i32;
        let next = (current + step).rem_euclid(types.len() as i32) as usize;
        self.settings.world_type = types[next].id.clone();
        self.settings.biome = None;
        self.settings.flat = None;
        self.layer = 0;
    }

    pub fn value(&self, slider: Slider) -> i32 {
        let s = &self.settings;
        match slider {
            Slider::BiomeSize => i32::from(s.biome_size),
            Slider::SeaLevel => s.sea_level.unwrap_or(self.catalog.sea_level),
            Slider::Caves => i32::from(s.caves),
            Slider::Ores => i32::from(s.ores),
        }
    }

    pub fn is_default(&self, slider: Slider) -> bool {
        let default = match slider {
            Slider::SeaLevel => self.catalog.sea_level,
            _ => 100,
        };
        self.value(slider) == default
    }

    /// Position of the slider, from 0 to 1.
    pub fn fraction(&self, slider: Slider) -> f32 {
        let range = slider.range();
        let span = (range.end() - range.start()).max(1);
        (self.value(slider) - range.start()) as f32 / span as f32
    }

    pub fn step(&mut self, slider: Slider, steps: i32) {
        self.set(slider, self.value(slider) + steps * slider.step());
    }

    /// Clicked or dragged to `fraction` of its width: the nearest step.
    pub fn set_fraction(&mut self, slider: Slider, fraction: f32) {
        let range = slider.range();
        let raw =
            *range.start() as f32 + fraction.clamp(0.0, 1.0) * (range.end() - range.start()) as f32;
        let snapped = ((raw / slider.step() as f32).round() as i32) * slider.step();
        self.set(slider, snapped);
    }

    fn set(&mut self, slider: Slider, value: i32) {
        let range = slider.range();
        let value = value.clamp(*range.start(), *range.end());
        let percent = value as u16;
        let s = &mut self.settings;
        match slider {
            Slider::BiomeSize => s.biome_size = percent,
            Slider::Caves => s.caves = percent,
            Slider::Ores => s.ores = percent,
            Slider::SeaLevel => {
                s.sea_level = (value != self.catalog.sea_level).then_some(value);
            }
        }
    }

    /// The biome used everywhere, for types that force one.
    pub fn biome(&self) -> Option<&Identifier> {
        self.settings
            .biome
            .as_ref()
            .or(self.world_type()?.biome.as_ref())
    }

    pub fn set_biome(&mut self, biome: Identifier) {
        let default = self.world_type().and_then(|t| t.biome.as_ref());
        self.settings.biome = (default != Some(&biome)).then_some(biome);
    }

    /// Layers of a flat type, from the bottom up (`None` for other types).
    pub fn flat(&self) -> Option<&FlatSettings> {
        self.settings
            .flat
            .as_ref()
            .or(self.world_type()?.flat.as_ref())
    }

    pub fn layers(&self) -> &[FlatLayer] {
        self.flat().map_or(&[], |flat| &flat.layers)
    }

    /// A new layer of `block` just above the selected one, then selected.
    pub fn add_layer(&mut self, block: Identifier) {
        let above = (self.layer + 1).min(self.layers().len());
        self.change_layers(|layers| {
            if layers.len() < MAX_FLAT_LAYERS {
                layers.insert(
                    above,
                    FlatLayer {
                        block,
                        thickness: 1,
                    },
                );
            }
        });
        self.layer = above.min(self.layers().len().saturating_sub(1));
    }

    /// The last layer stays: a flat world needs one.
    pub fn remove_layer(&mut self) {
        let layer = self.layer;
        self.change_layers(|layers| {
            if layers.len() > 1 && layer < layers.len() {
                layers.remove(layer);
            }
        });
        self.layer = self.layer.min(self.layers().len().saturating_sub(1));
    }

    /// Swaps the selected layer with the one above (`up`) or below; it stays selected.
    pub fn move_layer(&mut self, up: bool) {
        let layer = self.layer;
        let Some(other) = (if up {
            layer.checked_add(1)
        } else {
            layer.checked_sub(1)
        })
        .filter(|&other| other < self.layers().len()) else {
            return;
        };
        self.change_layers(|layers| layers.swap(layer, other));
        self.layer = other;
    }

    pub fn change_thickness(&mut self, layer: usize, steps: i32) {
        self.change_layers(|layers| {
            if let Some(layer) = layers.get_mut(layer) {
                let thickness = i32::from(layer.thickness) + steps;
                layer.thickness = thickness.clamp(1, i32::from(MAX_THICKNESS)) as u16;
            }
        });
    }

    pub fn set_block(&mut self, block: Identifier) {
        let layer = self.layer;
        self.change_layers(|layers| {
            if let Some(layer) = layers.get_mut(layer) {
                layer.block = block;
            }
        });
    }

    fn change_layers(&mut self, change: impl FnOnce(&mut Vec<FlatLayer>)) {
        let Some(mut flat) = self.flat().cloned() else {
            return;
        };
        change(&mut flat.layers);
        let flat = flat.bounded();
        let default = self.world_type().and_then(|t| t.flat.as_ref());
        self.settings.flat = (default != Some(&flat)).then_some(flat);
    }

    pub fn code(&self) -> String {
        self.settings.to_code()
    }

    /// Replaces the settings with those of `code`, if this game knows all it names.
    pub fn apply_code(&mut self, code: &str) -> Result<(), Refused> {
        let settings = WorldgenSettings::from_code(code).map_err(|e| Refused {
            message: e.message_id(),
            id: None,
        })?;
        let world_type = self
            .catalog
            .world_type(&settings.world_type)
            .ok_or_else(|| Refused::unknown("world-settings-unknown-type", &settings.world_type))?;
        if let Some(biome) = settings
            .biome
            .as_ref()
            .filter(|b| !self.catalog.biomes.contains(b))
        {
            return Err(Refused::unknown("world-settings-unknown-biome", biome));
        }
        let layers = settings.flat.iter().flat_map(|flat| &flat.layers);
        if let Some(layer) = layers.clone().find(|l| !self.is_known_block(&l.block)) {
            return Err(Refused::unknown(
                "world-settings-unknown-block",
                &layer.block,
            ));
        }
        if settings.flat.as_ref().is_some_and(|f| f.layers.is_empty()) {
            return Err(Refused {
                message: "world-settings-no-layers",
                id: None,
            });
        }
        let defaults = (world_type.biome.clone(), world_type.flat.clone());
        self.settings = settings;
        if self.settings.biome == defaults.0 {
            self.settings.biome = None;
        }
        if self.settings.flat == defaults.1 {
            self.settings.flat = None;
        }
        self.layer = 0;
        Ok(())
    }

    fn is_known_block(&self, block: &Identifier) -> bool {
        self.blocks.contains(block)
    }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
