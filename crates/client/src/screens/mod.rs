//! Everything drawn over the world: the HUD in play, menus on demand, the title screen.

mod console;
mod draw;
mod editor;
mod handling;
mod hud;
mod inventory;
mod keyboard;
mod menu;
mod options;
mod pause;
mod title;
mod worlds;

pub use menu::UiAction;

use crate::gameplay::Gameplay;
use crate::input::{Action, InputDevice, KeyNames};
use crate::render::{Gpu, UiPipeline};
use crate::settings::{FontChoice, Settings};
use crate::ui::{Hit, ICON_SIZE, Rect, ScreenModel, UiAssets, UiAtlas};
use console::ConsoleLog;
use draw::key_rows;
use keyboard::{Keyboard, KeyboardModel};
use menu::{Effect, Menu, Shared};
use std::path::PathBuf;
use strata_core::content::Content;
use strata_core::texture::RgbaImage;
use strata_protocol::WorldSummary;
use strata_protocol::commands::Feedback;
use winit::keyboard::KeyCode;

/// Where the interface is drawn: over the frame in `view`.
pub struct UiFrame<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub size: (u32, u32),
    /// F3 overlay lines, drawn over the HUD.
    pub debug: &'a [String],
}

/// Largest interface scale the automatic setting may pick.
const MAX_AUTO_SCALE: u32 = 4;

pub struct Screens {
    assets: UiAssets,
    atlas: UiAtlas,
    pipeline: UiPipeline,
    pub settings: Settings,
    settings_path: PathBuf,
    stack: Vec<Menu>,
    in_world: bool,
    focus: Option<Hit>,
    hits: Vec<Hit>,
    scale: u32,
    /// Text field receiving the physical keyboard.
    editing: Option<String>,
    keyboard: Option<Keyboard>,
    keyboard_hits: Vec<Hit>,
    worlds: Vec<WorldSummary>,
    icons: Vec<Option<Rect>>,
    /// Slider or HUD element followed by the mouse while the button is held.
    dragging: Option<Hit>,
    last_cursor: Option<(i32, i32)>,
    effects: Vec<Effect>,
    /// Settings changed but not written yet (see `handle`).
    settings_dirty: bool,
    key_names: KeyNames,
    /// Keyboard and gamepad label of each action, rebuilt when bindings, language or
    /// learned key names change.
    key_labels: Vec<(String, String)>,
    labels_stale: bool,
    /// Last device used: prompts show its keys or buttons.
    device: InputDevice,
    /// Announced on the title screen.
    pub newer_version: Option<String>,
    /// The font drawn into the atlas and its density, redrawn when either changes.
    atlas_font: (FontChoice, u32),
    console: ConsoleLog,
    /// Resource packs found on disk, for the options.
    pub available_packs: Vec<String>,
}

impl Screens {
    pub fn new(
        gpu: &Gpu,
        color_format: wgpu::TextureFormat,
        assets: UiAssets,
        content: &Content,
        settings: Settings,
        settings_path: PathBuf,
    ) -> Self {
        let font = settings.interface.font;
        let atlas = UiAtlas::build(content, assets.font_for(font).font(1));
        let pipeline = UiPipeline::new(gpu, color_format, &atlas.image);
        Self {
            assets,
            atlas,
            pipeline,
            settings,
            settings_path,
            stack: vec![Menu::Title],
            in_world: false,
            focus: None,
            hits: Vec::new(),
            scale: 1,
            editing: None,
            keyboard: None,
            keyboard_hits: Vec::new(),
            worlds: Vec::new(),
            icons: Vec::new(),
            dragging: None,
            last_cursor: None,
            effects: Vec::new(),
            settings_dirty: false,
            key_names: KeyNames::default(),
            key_labels: Vec::new(),
            labels_stale: true,
            device: InputDevice::KeyboardMouse,
            newer_version: None,
            atlas_font: (font, 1),
            console: ConsoleLog::default(),
            available_packs: Vec::new(),
        }
    }

    /// Back to the title screen (after leaving a world).
    pub fn show_title(&mut self) {
        self.in_world = false;
        self.stack = vec![Menu::Title];
        self.reset_focus();
    }

    /// A world just started: no menu, the HUD only.
    pub fn enter_world(&mut self) {
        self.in_world = true;
        self.stack.clear();
        self.reset_focus();
    }

    /// Shows the world list (replacing any open one) with each world's capture.
    pub fn open_worlds(&mut self, gpu: &Gpu, worlds: Vec<WorldSummary>) {
        self.icons = worlds
            .iter()
            .enumerate()
            .map(|(slot, world)| self.upload_icon(gpu, slot, world.icon.as_deref()))
            .collect();
        let selected = (!worlds.is_empty()).then_some(0);
        self.worlds = worlds;
        self.stack.truncate(1);
        self.stack.push(Menu::Worlds {
            selected,
            scroll: 0,
        });
        self.reset_focus();
    }

    /// Interface files read again (resource reload); the font and atlas are redrawn.
    pub fn replace_assets(
        &mut self,
        gpu: &Gpu,
        color_format: wgpu::TextureFormat,
        assets: UiAssets,
        content: &Content,
    ) {
        self.assets = assets;
        self.atlas = UiAtlas::build(content, self.assets.font.font(1));
        self.atlas_font = (FontChoice::Pixel, 1);
        self.pipeline = UiPipeline::new(gpu, color_format, &self.atlas.image);
        self.labels_stale = true;
    }

    /// World captures in the atlas are lost: lists upload them again when opened.
    fn refresh_font(&mut self, gpu: &Gpu, content: &Content) {
        let choice = self.settings.interface.font;
        let source = self.assets.font_for(choice);
        let density = if source.is_scalable() { self.scale } else { 1 };
        if (choice, density) == self.atlas_font {
            return;
        }
        self.atlas_font = (choice, density);
        self.atlas = UiAtlas::build(content, source.font(density));
        self.pipeline.write_atlas(gpu, 0, 0, &self.atlas.image);
    }

    /// Server answers to console commands.
    pub fn print_feedback(&mut self, feedback: &[Feedback], content: &Content) {
        let language = &self.settings.interface.language;
        for answer in feedback {
            self.console.feedback(answer, content, language);
        }
    }

    pub fn show_crash(&mut self) {
        self.stack.push(Menu::Crash);
        self.reset_focus();
    }

    pub fn show_notice(&mut self, key: &'static str) {
        self.stack.push(Menu::Notice { key });
        self.reset_focus();
    }

    /// A menu has the cursor (the title screen counts as one).
    pub fn is_menu_open(&self) -> bool {
        !self.in_world || !self.stack.is_empty()
    }

    /// Whether the open menus stop the world (see `Menu::pauses`).
    pub fn wants_pause(&self) -> bool {
        self.stack.iter().any(|menu| menu.pauses(&self.settings))
    }

    fn upload_icon(&self, gpu: &Gpu, slot: usize, png: Option<&[u8]>) -> Option<Rect> {
        let place = UiAtlas::world_icon_slot(slot)?;
        let image = decode_icon(png?)?;
        if (image.width(), image.height()) != (place.w, place.h) {
            return None;
        }
        self.pipeline
            .write_atlas(gpu, place.x as u32, place.y as u32, &image);
        Some(place)
    }

    fn reset_focus(&mut self) {
        self.focus = None;
        self.hits.clear();
        self.editing = None;
        self.keyboard = None;
        self.keyboard_hits.clear();
        self.dragging = None;
        self.last_cursor = None;
    }

    /// Changes settings from outside the menus (F7/F8); written like menu changes.
    pub fn change_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.settings);
        self.settings_dirty = true;
    }

    /// A key typed something: its printed name may now be known.
    pub fn learn_key(&mut self, code: KeyCode, text: &str) {
        self.labels_stale |= self.key_names.learn(code, text);
    }

    fn refresh_labels(&mut self, content: &Content) {
        if self.labels_stale {
            let bindings = &self.settings.controls.bindings;
            let language = &self.settings.interface.language;
            self.key_labels = self
                .key_names
                .table(bindings, &content.translations, language);
            self.labels_stale = false;
        }
    }

    /// Runs `f` on the model of the menu on top, if any.
    fn with_model<R>(
        &mut self,
        game: Option<&mut Gameplay>,
        content: &Content,
        f: impl FnOnce(&mut dyn ScreenModel) -> R,
    ) -> Option<R> {
        let menu = self.stack.last_mut()?;
        let labels = &self.key_labels;
        let key_labels = |action: Action| labels.get(action.index()).cloned().unwrap_or_default();
        let shared = Shared {
            content,
            settings: &mut self.settings,
            worlds: &self.worlds,
            icons: &self.icons,
            hotbar: game.map(|g| &mut g.hotbar),
            focus: self.focus.as_ref().map(|f| (f.id.clone(), f.index)),
            key_labels: &key_labels,
            newer_version: self.newer_version.as_deref(),
            console: &self.console,
            packs: &self.available_packs,
            effects: &mut self.effects,
        };
        let mut model = menu::model(menu, shared);
        Some(f(model.as_mut()))
    }

    /// Presses key `index` of the on-screen keyboard: (field, text to type, closed).
    fn press_key(&mut self, content: &Content, index: usize) -> Option<(String, String, bool)> {
        let keyboard = self.keyboard.as_mut()?;
        let (mut typed, mut done) = (String::new(), false);
        let mut model = KeyboardModel {
            keyboard,
            rows: key_rows(&self.assets.screens.keyboard),
            translations: &content.translations,
            language: &self.settings.interface.language,
            typed: &mut typed,
            done: &mut done,
        };
        model.activate("keys", index);
        let field = model.keyboard.field.clone();
        Some((field, typed, done))
    }
}

fn decode_icon(png_bytes: &[u8]) -> Option<RgbaImage> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    if (info.width, info.height) != ICON_SIZE {
        return None;
    }
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buffer.truncate(info.buffer_size());
    RgbaImage::from_bytes(info.width, info.height, buffer)
}

/// Interface texts chosen in code rather than in screen files.
#[cfg(test)]
const CODE_KEYS: &[&str] = &[
    inventory::TAB_ALL_KEY,
    "ui-editor-sample",
    "ui-error-incompatible",
    "ui-error-unavailable",
    "ui-error-broken",
    "ui-reload-failed",
    "ui-reload-blocks-changed",
    "ui-update-available",
    "ui-version",
    "ui-scale-times",
    "ui-world-detail",
    "ui-world-too-old",
    "ui-world-too-new",
    "ui-world-broken",
    "ui-worlds-empty",
    "ui-mode-creative",
    "ui-new-world-mode",
    "ui-new-world-default-name",
    "ui-delete-question",
    "ui-play-time-hours",
    "ui-play-time-minutes",
    "ui-on",
    "ui-off",
    "ui-scale-auto",
    "ui-key-erase",
    "ui-key-shift",
    "ui-key-space",
    "ui-key-done",
];

#[cfg(test)]
mod tests {
    use crate::input::translation_keys as key_translation_keys;
    use crate::ui::UiAssets;
    use std::path::Path;
    use strata_core::content::Content;
    use strata_core::pack::PackSet;

    #[test]
    fn every_interface_text_is_translated() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let mut errors = Vec::new();
        let packs = PackSet::discover(&assets.join("packs"), &mut errors);
        let ui = UiAssets::load(&assets.join("ui"), &packs, &mut errors).unwrap();
        assert!(errors.is_empty(), "{errors:#?}");
        let (content, _) = Content::load(&packs, &PackSet::default());
        let mut keys = ui.translation_keys();
        keys.extend(super::CODE_KEYS.iter().map(|k| (*k).to_owned()));
        keys.extend(key_translation_keys());
        keys.extend(super::options::translation_keys());
        keys.extend(crate::debug_overlay::translation_keys());
        keys.extend(super::console::translation_keys());
        let missing = content.translations.missing(&keys);
        assert!(missing.is_empty(), "missing translations: {missing:#?}");
    }
}
