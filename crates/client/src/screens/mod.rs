//! Everything drawn over the world: the HUD in play, menus on demand, the title screen.

mod anvil;
mod bench;
mod console;
mod creation;
mod draw;
mod editor;
mod effects;
mod handling;
mod hud;
mod inventory;
mod journal;
mod keyboard;
mod menu;
mod options;
mod pause;
mod pointer;
mod slot_input;
mod slots;
mod station_lists;
mod system;
mod title;
mod worlds;

pub use menu::UiAction;
pub use options::{REBINDABLE, action_key, catalog as settings_catalog};

use crate::gameplay::Gameplay;
use crate::input::{Action, InputDevice, KeyNames};
use crate::render::{Gpu, UiPipeline};
use crate::settings::{FontChoice, Settings};
use crate::ui::{Hit, HoldRepeat, Rect, ScreenModel, UiAssets, UiAtlas};
use console::ConsoleLog;
pub use creation::GenerationSources;
use draw::key_rows;
use keyboard::{Keyboard, KeyboardModel};
use menu::{Effect, Menu, Shared};
use pointer::ValueRepeat;
use std::path::PathBuf;
use strata_core::content::Content;
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
    /// Sounds heard lately (empty when subtitles are off).
    pub subtitles: &'a [crate::audio::Subtitle],
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
    /// ← → held on a value.
    repeat: ValueRepeat,
    /// Arrows held in the HUD editor (x, y).
    editor_repeat: [HoldRepeat; 2],
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
    pub generation: GenerationSources,
    /// Opened on first use; kept, since on X11 copied text lives as long as it does.
    clipboard: Option<arboard::Clipboard>,
    /// Request number of the map in the atlas (`None` after the atlas is rebuilt).
    preview_uploaded: Option<u64>,
    slot_input: slot_input::SlotInput,
    /// Where the mouse points (interface pixels), unless a gamepad is in use.
    pointer: Option<(i32, i32)>,
    /// The window of the menu on top: clicks outside it throw the carried stack.
    window: Option<Rect>,
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
            repeat: ValueRepeat::default(),
            editor_repeat: Default::default(),
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
            generation: GenerationSources::default(),
            clipboard: None,
            preview_uploaded: None,
            slot_input: Default::default(),
            pointer: None,
            window: None,
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

    /// Interface files read again (resource reload), with their atlas and pipeline.
    pub fn replace_assets(&mut self, assets: UiAssets, atlas: UiAtlas, pipeline: UiPipeline) {
        self.assets = assets;
        self.atlas = atlas;
        self.atlas_font = (FontChoice::Pixel, 1);
        self.pipeline = pipeline;
        self.preview_uploaded = None;
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
        self.preview_uploaded = None;
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
        self.stack.push(Menu::Notice { key, id: None });
        self.reset_focus();
    }

    pub fn show_error(&mut self, error: &crate::launcher::LaunchError) {
        let (key, id) = error.message();
        self.stack.push(Menu::Notice { key, id });
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
        let parts = menu::game_parts(game);
        let labels = &self.key_labels;
        let key_labels = |action: Action| labels.get(action.index()).cloned().unwrap_or_default();
        let shared = Shared {
            content,
            settings: &mut self.settings,
            worlds: &self.worlds,
            icons: &self.icons,
            inventory: parts.inventory,
            container: parts.container,
            crafting: parts.crafting,
            focus: self.focus.as_ref().map(|f| (f.id.clone(), f.index)),
            key_labels: &key_labels,
            newer_version: self.newer_version.as_deref(),
            console: &self.console,
            packs: &self.available_packs,
            generation: &self.generation,
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

#[cfg(test)]
mod translation_tests;
