//! Home-made pixel-art interface: screens described in RON (`assets/ui/`), drawn as quads
//! from one atlas, navigable with mouse, keyboard and gamepad.

mod atlas;
mod def;
mod focus;
mod model;
mod painter;
mod rect;
mod theme;
mod widgets;

pub use atlas::UiAtlas;
pub use def::ScreenDef;
pub use focus::{Direction, navigate};
pub use model::{Hit, ScreenModel};
pub use painter::{Painter, UiVertex, auto_scale};
pub use rect::Rect;
pub use theme::Theme;
pub use widgets::{UiContext, draw_screen};

use std::path::Path;
use strata_core::font::Font;
use strata_core::pack::{PackError, PackSet, read_ron};

/// Interface files: theme, screen layouts and the font.
pub struct UiAssets {
    pub theme: Theme,
    pub hud: ScreenDef,
    pub inventory: ScreenDef,
    pub font: Font,
}

impl UiAssets {
    /// `None` (with the errors) if a required file is missing or broken.
    pub fn load(ui_dir: &Path, packs: &PackSet, errors: &mut Vec<PackError>) -> Option<Self> {
        let theme = read_or_report(&ui_dir.join("theme.ron"), errors);
        let hud = read_or_report(&ui_dir.join("hud.ron"), errors);
        let inventory = read_or_report(&ui_dir.join("creative_inventory.ron"), errors);
        let font = Font::load(packs, "pixel", errors);
        Some(Self {
            theme: theme?,
            hud: hud?,
            inventory: inventory?,
            font: font?,
        })
    }

    pub fn translation_keys(&self) -> Vec<String> {
        let mut keys = self.hud.translation_keys();
        keys.extend(self.inventory.translation_keys());
        keys
    }
}

fn read_or_report<T: serde::de::DeserializeOwned>(
    path: &Path,
    errors: &mut Vec<PackError>,
) -> Option<T> {
    read_ron(path).map_err(|e| errors.push(e)).ok()
}
