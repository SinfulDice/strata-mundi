//! Interface choices kept between sessions in `config/interface.toml` (the full settings
//! file and its screen arrive at milestone 8).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use strata_core::files::write_atomic;
use strata_core::lang::LANGUAGES;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceSettings {
    pub language: String,
    /// 0 = automatic (largest whole scale that fits), otherwise ×1, ×2…
    pub scale: u32,
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
            pause_in_menus: false,
            hud: BTreeMap::new(),
        }
    }
}

/// Largest manual interface scale offered.
pub const MAX_SCALE: u32 = 6;

impl InterfaceSettings {
    /// Defaults when the file is missing; a broken file is reported and ignored.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let mut settings: Self = toml::from_str(&text).unwrap_or_else(|error| {
            tracing::warn!(path = %path.display(), %error, "interface settings ignored");
            Self::default()
        });
        if !LANGUAGES.contains(&settings.language.as_str()) {
            settings.language = Self::default().language;
        }
        settings.scale = settings.scale.min(MAX_SCALE);
        settings
    }

    pub fn save(&self, path: &Path) {
        let text = toml::to_string_pretty(self);
        let written = text
            .map_err(|e| e.to_string())
            .and_then(|text| write_atomic(path, text.as_bytes()).map_err(|e| e.to_string()));
        if let Err(error) = written {
            tracing::warn!(path = %path.display(), %error, "interface settings not saved");
        }
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_bad_values_fall_back() {
        let dir = std::env::temp_dir().join(format!("strata-iface-{}", std::process::id()));
        let path = dir.join("interface.toml");
        let mut settings = InterfaceSettings::default();
        settings.hud.insert("hotbar".into(), (4, -10));
        settings.pause_in_menus = true;
        settings.save(&path);
        assert_eq!(InterfaceSettings::load(&path), settings);
        std::fs::write(&path, "language = \"xx\"\nscale = 99\n").unwrap();
        let fixed = InterfaceSettings::load(&path);
        assert_eq!((fixed.language.as_str(), fixed.scale), ("fr", MAX_SCALE));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
