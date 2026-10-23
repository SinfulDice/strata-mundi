//! Home-made pixel-art interface: screens described in RON (`assets/ui/`), drawn as quads
//! from one atlas, navigable with mouse, keyboard and gamepad.

mod atlas;
mod def;
mod focus;
mod model;
mod painter;
mod rect;
pub mod rich;
mod theme;
mod widgets;

pub use atlas::{ICON_SIZE, UiAtlas};
pub use def::{Node, ScreenDef};
pub use focus::{Direction, navigate};
pub use model::{Hit, HitKind, ListRow, LogLine, OptionRow, ScreenModel, Tone};
pub use painter::{Painter, UiVertex, auto_scale};
pub use rect::Rect;
pub use theme::Theme;
pub use widgets::{UiContext, draw_screen};

use crate::settings::FontChoice;
use std::path::Path;
use strata_core::font::FontSource;
use strata_core::pack::{PackError, PackSet, read_ron};

/// Every screen layout, one file each in `assets/ui/`.
pub struct ScreenDefs {
    pub hud: ScreenDef,
    pub inventory: ScreenDef,
    pub title: ScreenDef,
    pub worlds: ScreenDef,
    pub create_world: ScreenDef,
    pub confirm_delete: ScreenDef,
    pub pause: ScreenDef,
    pub options: ScreenDef,
    pub hud_editor: ScreenDef,
    pub keyboard: ScreenDef,
    pub notice: ScreenDef,
    pub crash: ScreenDef,
    pub console: ScreenDef,
}

impl ScreenDefs {
    fn all(&self) -> [&ScreenDef; 13] {
        [
            &self.hud,
            &self.inventory,
            &self.title,
            &self.worlds,
            &self.create_world,
            &self.confirm_delete,
            &self.pause,
            &self.options,
            &self.hud_editor,
            &self.keyboard,
            &self.notice,
            &self.crash,
            &self.console,
        ]
    }
}

/// Interface files: theme, screen layouts and the fonts.
pub struct UiAssets {
    pub theme: Theme,
    pub screens: ScreenDefs,
    pub font: FontSource,
    /// Accessibility option; the pixel font stays in use if it is missing or broken.
    pub readable_font: Option<FontSource>,
}

impl UiAssets {
    /// The font for `choice`, falling back to the pixel font.
    pub fn font_for(&self, choice: FontChoice) -> &FontSource {
        match (choice, &self.readable_font) {
            (FontChoice::Readable, Some(font)) => font,
            _ => &self.font,
        }
    }

    /// `None` (with the errors) if a required file is missing or broken.
    pub fn load(ui_dir: &Path, packs: &PackSet, errors: &mut Vec<PackError>) -> Option<Self> {
        let mut read =
            |name: &str| read_or_report::<ScreenDef>(&ui_dir.join(format!("{name}.ron")), errors);
        let screens = ScreenDefs {
            hud: read("hud")?,
            inventory: read("creative_inventory")?,
            title: read("title")?,
            worlds: read("worlds")?,
            create_world: read("create_world")?,
            confirm_delete: read("confirm_delete")?,
            pause: read("pause")?,
            options: read("options")?,
            hud_editor: read("hud_editor")?,
            keyboard: read("keyboard")?,
            notice: read("notice")?,
            crash: read("crash")?,
            console: read("console")?,
        };
        let theme = read_or_report(&ui_dir.join("theme.ron"), errors);
        let font = FontSource::load(packs, "pixel", errors);
        let mut optional_errors = Vec::new();
        let readable_font = FontSource::load(packs, "readable", &mut optional_errors);
        for error in optional_errors {
            tracing::warn!(%error, "readable font unavailable");
        }
        Some(Self {
            theme: theme?,
            screens,
            font: font?,
            readable_font,
        })
    }

    pub fn translation_keys(&self) -> Vec<String> {
        self.screens
            .all()
            .iter()
            .flat_map(|screen| screen.translation_keys())
            .collect()
    }
}

fn read_or_report<T: serde::de::DeserializeOwned>(
    path: &Path,
    errors: &mut Vec<PackError>,
) -> Option<T> {
    read_ron(path).map_err(|e| errors.push(e)).ok()
}
