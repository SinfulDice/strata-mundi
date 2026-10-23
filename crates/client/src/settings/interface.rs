//! Interface and accessibility settings.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strata_core::lang::LANGUAGES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontChoice {
    /// The pixel font of the game (`fonts/pixel.ron`).
    #[default]
    Pixel,
    /// A plain, very readable font (`fonts/readable.ttf`).
    Readable,
}

/// Colour filters for colour blindness, applied to the whole image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorFilter {
    #[default]
    None,
    /// Red-blind.
    Protanopia,
    /// Green-blind (the most common).
    Deuteranopia,
    /// Blue-blind.
    Tritanopia,
}

impl ColorFilter {
    pub const ALL: [ColorFilter; 4] = [
        Self::None,
        Self::Protanopia,
        Self::Deuteranopia,
        Self::Tritanopia,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceSettings {
    pub language: String,
    /// 0 = automatic (largest whole scale that fits), otherwise ×1, ×2…
    pub scale: u32,
    pub font: FontChoice,
    pub color_filter: ColorFilter,
    /// No camera sway or zoom effects (sprint zoom).
    pub reduce_camera_effects: bool,
    /// Stop the world in every menu, not only in the pause menu (off, like Minecraft).
    pub pause_in_menus: bool,
    /// HUD elements moved in the editor: offset from their anchor, by element id.
    pub hud: BTreeMap<String, (i32, i32)>,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            language: "fr".to_owned(),
            scale: 0,
            font: FontChoice::default(),
            color_filter: ColorFilter::default(),
            reduce_camera_effects: false,
            pause_in_menus: false,
            hud: BTreeMap::new(),
        }
    }
}

/// Largest manual interface scale offered.
pub const MAX_SCALE: u32 = 6;

impl InterfaceSettings {
    /// Values a hand-edited file may get wrong.
    pub fn sanitize(&mut self) {
        if !LANGUAGES.contains(&self.language.as_str()) {
            self.language = Self::default().language;
        }
        self.scale = self.scale.min(MAX_SCALE);
    }

    /// The next language in `LANGUAGES`, for the options screen.
    pub fn cycle_language(&mut self) {
        let index = LANGUAGES
            .iter()
            .position(|l| *l == self.language)
            .map_or(0, |i| (i + 1) % LANGUAGES.len());
        self.language = LANGUAGES[index].to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_values_fall_back() {
        let mut settings = InterfaceSettings {
            language: "xx".into(),
            scale: 99,
            ..InterfaceSettings::default()
        };
        settings.sanitize();
        assert_eq!(
            (settings.language.as_str(), settings.scale),
            ("fr", MAX_SCALE)
        );
        settings.cycle_language();
        assert_ne!(settings.language, "fr");
    }
}
