//! Every player setting, kept in `config/settings.toml` (edited in the options screen or
//! by hand). A missing or broken file gives the defaults; unknown values fall back.

mod controls;
mod game;
mod graphics;
mod interface;

pub use controls::{ControlSettings, DEAD_ZONE, SENSITIVITY};
pub use game::GameSettings;
pub use graphics::{
    ANISOTROPY, AUTO_THREADS, FOV, GraphicsSettings, Preset, RENDER_DISTANCE, RENDER_SCALE,
    VERTICAL_DISTANCE, default_worker_threads,
};
pub use interface::{ColorFilter, FontChoice, InterfaceSettings, MAX_SCALE};

use serde::{Deserialize, Serialize};
use std::path::Path;
use strata_core::files::write_atomic;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The preset the graphics come from; `None` once one of them was changed by hand.
    pub preset: Option<Preset>,
    pub graphics: GraphicsSettings,
    pub interface: InterfaceSettings,
    pub controls: ControlSettings,
    pub game: GameSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            preset: Some(Preset::High),
            graphics: GraphicsSettings::for_preset(Preset::High),
            interface: InterfaceSettings::default(),
            controls: ControlSettings::default(),
            game: GameSettings::default(),
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let mut settings: Self = toml::from_str(&text).unwrap_or_else(|error| {
            tracing::warn!(path = %path.display(), %error, "settings ignored");
            Self::default()
        });
        settings.interface.sanitize();
        settings.graphics.sanitize();
        settings.controls.sanitize();
        settings
    }

    pub fn save(&self, path: &Path) {
        let written = toml::to_string_pretty(self)
            .map_err(|e| e.to_string())
            .and_then(|text| write_atomic(path, text.as_bytes()).map_err(|e| e.to_string()));
        if let Err(error) = written {
            tracing::warn!(path = %path.display(), %error, "settings not saved");
        }
    }

    pub fn apply_preset(&mut self, preset: Preset) {
        self.preset = Some(preset);
        self.graphics = GraphicsSettings::for_preset(preset);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_toml() {
        let dir = std::env::temp_dir().join(format!("strata-settings-{}", std::process::id()));
        let path = dir.join("settings.toml");
        let mut settings = Settings::default();
        settings.apply_preset(Preset::Ds);
        settings.interface.hud.insert("hotbar".into(), (4, -10));
        settings.controls.invert_y = true;
        settings.controls.bindings.rebind(
            crate::input::Action::Jump,
            crate::input::Binding::Key(winit::keyboard::KeyCode::KeyJ),
        );
        settings.save(&path);
        assert_eq!(Settings::load(&path), settings);
        std::fs::write(&path, "[graphics]\nfov = \"wide\"\n").unwrap();
        assert_eq!(
            Settings::load(&path),
            Settings::default(),
            "broken file: defaults"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
