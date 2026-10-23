//! The options screen: one tab per family of settings, one row per setting, with a short
//! description of what it does to performance. Changes apply at once.

use super::menu::{Effect, Menu, Shared};
use crate::input::{Action, Binding};
use crate::settings::MAX_SCALE;
use crate::settings::{
    AUTO_THREADS, ColorFilter, FontChoice, Preset, Settings, default_worker_threads,
};
use crate::ui::{OptionRow, ScreenModel};

/// Rows visible at once (matches `options.ron`).
pub const VISIBLE_ROWS: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Preset,
    RenderDistance,
    VerticalDistance,
    RenderScale,
    FpsLimit,
    Vsync,
    Fov,
    AmbientOcclusion,
    Fog,
    Mipmaps,
    Anisotropy,
    MeshThreads,
    WorldThreads,
    Language,
    Scale,
    Font,
    ColorFilter,
    ReduceCamera,
    PauseInMenus,
    BlockName,
    EditHud,
    MouseSensitivity,
    StickSensitivity,
    InvertY,
    DeadZone,
    AutoJump,
    StepUp,
    ResetKeys,
    CheckUpdates,
    Autosave,
    Backups,
    IdleUnfocused,
}

const GRAPHICS: &[Setting] = &[
    Setting::Preset,
    Setting::RenderDistance,
    Setting::VerticalDistance,
    Setting::RenderScale,
    Setting::FpsLimit,
    Setting::Vsync,
    Setting::Fov,
    Setting::AmbientOcclusion,
    Setting::Fog,
    Setting::Mipmaps,
    Setting::Anisotropy,
    Setting::MeshThreads,
    Setting::WorldThreads,
];
const INTERFACE: &[Setting] = &[
    Setting::Language,
    Setting::Scale,
    Setting::Font,
    Setting::ColorFilter,
    Setting::ReduceCamera,
    Setting::PauseInMenus,
    Setting::BlockName,
    Setting::EditHud,
];
const CONTROLS: &[Setting] = &[
    Setting::MouseSensitivity,
    Setting::StickSensitivity,
    Setting::InvertY,
    Setting::DeadZone,
    Setting::AutoJump,
    Setting::StepUp,
];
const GAME: &[Setting] = &[
    Setting::CheckUpdates,
    Setting::Autosave,
    Setting::Backups,
    Setting::IdleUnfocused,
];
/// Tab titles; the keys tab (index 3) lists `REBINDABLE` actions instead of settings.
const TABS: [&str; 5] = [
    "option-tab-graphics",
    "option-tab-interface",
    "option-tab-controls",
    "option-tab-keys",
    "option-tab-game",
];
const KEYS_TAB: usize = 3;

/// Actions the player can rebind (menu navigation keeps fixed keys so menus always work).
pub const REBINDABLE: [Action; 25] = [
    Action::MoveForward,
    Action::MoveBackward,
    Action::MoveLeft,
    Action::MoveRight,
    Action::Jump,
    Action::Sneak,
    Action::Sprint,
    Action::Attack,
    Action::Use,
    Action::PickBlock,
    Action::Inventory,
    Action::HotbarNext,
    Action::HotbarPrevious,
    Action::Hotbar1,
    Action::Hotbar2,
    Action::Hotbar3,
    Action::Hotbar4,
    Action::Hotbar5,
    Action::Hotbar6,
    Action::Hotbar7,
    Action::Hotbar8,
    Action::Hotbar9,
    Action::RenderDistanceDown,
    Action::RenderDistanceUp,
    Action::ToggleDebug,
];

const FPS_LIMITS: [u32; 6] = [0, 30, 60, 120, 144, 240];
const ANISOTROPY: [u32; 5] = [1, 2, 4, 8, 16];
const AUTOSAVE: [u32; 7] = [0, 1, 2, 5, 10, 15, 30];

#[derive(Debug, Default)]
pub struct OptionsState {
    pub tab: usize,
    pub scroll: usize,
    /// Waiting for the input to bind to this action.
    pub capturing: Option<Action>,
}

pub struct OptionsModel<'a> {
    pub state: &'a mut OptionsState,
    pub shared: Shared<'a>,
    /// Labels of the bound inputs, by action (keyboard, gamepad).
    pub key_labels: &'a dyn Fn(Action) -> (String, String),
}

/// What a row of the current tab stands for.
enum Row {
    Setting(Setting),
    Key(Action),
}

impl OptionsModel<'_> {
    fn rows(&self) -> Vec<Row> {
        match self.state.tab {
            KEYS_TAB => REBINDABLE
                .into_iter()
                .map(Row::Key)
                .chain(std::iter::once(Row::Setting(Setting::ResetKeys)))
                .collect(),
            tab => {
                let settings = match tab {
                    0 => GRAPHICS,
                    1 => INTERFACE,
                    2 => CONTROLS,
                    _ => GAME,
                };
                settings.iter().map(|&s| Row::Setting(s)).collect()
            }
        }
    }

    fn row_at(&self, visible_index: usize) -> Option<Row> {
        self.rows()
            .into_iter()
            .nth(self.state.scroll + visible_index)
    }

    fn describe(&self, row: &Row) -> OptionRow {
        match *row {
            Row::Key(action) => {
                let (keyboard, pad) = (self.key_labels)(action);
                OptionRow {
                    label: self.shared.tr(&action_key(action)),
                    value: format!("{keyboard} | {pad}"),
                    gauge: None,
                    highlight: self.state.capturing == Some(action),
                }
            }
            Row::Setting(setting) => {
                let (value, gauge) = value(setting, self.shared.settings, &self.shared);
                OptionRow {
                    label: self.shared.tr(&setting_key(setting)),
                    value,
                    gauge,
                    highlight: false,
                }
            }
        }
    }

    fn change(&mut self, setting: Setting, step: i32) {
        let effect = match setting {
            Setting::EditHud => Effect::Push(Menu::HudEditor { selected: None }),
            Setting::ResetKeys => {
                self.shared.settings.controls.bindings = Default::default();
                Effect::SettingsChanged
            }
            _ => {
                step_setting(setting, self.shared.settings, step);
                Effect::SettingsChanged
            }
        };
        self.shared.effects.push(effect);
    }
}

impl ScreenModel for OptionsModel<'_> {
    fn tabs(&self, _: &str) -> Vec<String> {
        TABS.iter().map(|key| self.shared.tr(key)).collect()
    }

    fn selected(&self, id: &str) -> Option<usize> {
        (id == "tabs").then_some(self.state.tab)
    }

    fn option_rows(&self, _: &str) -> Vec<OptionRow> {
        self.rows()
            .iter()
            .skip(self.state.scroll)
            .take(VISIBLE_ROWS)
            .map(|row| self.describe(row))
            .collect()
    }

    /// `description`: what the focused row does (and costs).
    fn text(&self, id: &str) -> Option<String> {
        let (focus_id, index) = self.shared.focus.as_ref()?;
        if id != "description" || focus_id != "options" {
            return None;
        }
        Some(match self.row_at(*index)? {
            Row::Key(_) if self.state.capturing.is_some() => self.shared.tr("option-key-waiting"),
            Row::Key(_) => self.shared.tr("option-key-help"),
            Row::Setting(setting) => self.shared.tr(&format!("{}-help", setting_key(setting))),
        })
    }

    fn scroll(&mut self, _: &str, rows: i32) {
        let max = self.rows().len().saturating_sub(VISIBLE_ROWS) as i32;
        self.state.scroll = (self.state.scroll as i32 + rows).clamp(0, max) as usize;
    }

    fn adjust(&mut self, _: &str, index: usize, step: i32, _: Option<f32>) {
        if let Some(Row::Setting(setting)) = self.row_at(index)
            && !matches!(setting, Setting::EditHud | Setting::ResetKeys)
        {
            self.change(setting, step);
        }
    }

    fn activate(&mut self, id: &str, index: usize) {
        match id {
            "tabs" => {
                *self.state = OptionsState {
                    tab: index.min(TABS.len() - 1),
                    ..OptionsState::default()
                };
            }
            "options" => match self.row_at(index) {
                Some(Row::Key(action)) => self.state.capturing = Some(action),
                Some(Row::Setting(setting)) => self.change(setting, 1),
                None => {}
            },
            "back" => self.shared.effects.push(Effect::Pop),
            _ => {}
        }
    }
}

/// Binds the captured action to `binding` (the next input pressed).
pub fn bind_captured(state: &mut OptionsState, settings: &mut Settings, binding: Binding) {
    if let Some(action) = state.capturing.take() {
        let bindings = &mut settings.controls.bindings;
        bindings.release(binding, &REBINDABLE, action);
        bindings.rebind(action, binding);
    }
}

fn setting_key(setting: Setting) -> String {
    kebab_key("option", &format!("{setting:?}"))
}

pub fn action_key(action: Action) -> String {
    kebab_key("action", &format!("{action:?}"))
}

/// `("action", "Hotbar1")` → `action-hotbar-1`.
fn kebab_key(prefix: &str, name: &str) -> String {
    let mut key = String::from(prefix);
    for ch in name.chars() {
        if ch.is_uppercase() || ch.is_ascii_digit() {
            key.push('-');
        }
        key.extend(ch.to_lowercase());
    }
    key
}

fn on_off(shared: &Shared<'_>, value: bool) -> String {
    shared.tr(if value { "ui-on" } else { "ui-off" })
}

fn threads_label(shared: &Shared<'_>, threads: usize) -> String {
    if threads == AUTO_THREADS {
        shared.tr("option-auto")
    } else {
        threads.to_string()
    }
}

/// Text of the value and, for ranges, the gauge position.
fn value(setting: Setting, s: &Settings, shared: &Shared<'_>) -> (String, Option<f32>) {
    let (g, i, c, game) = (&s.graphics, &s.interface, &s.controls, &s.game);
    let gauge =
        |v: u32, min: u32, max: u32| Some((v.saturating_sub(min)) as f32 / (max - min) as f32);
    match setting {
        Setting::Preset => (
            shared.tr(&s
                .preset
                .map_or("preset-custom".to_owned(), |p| format!("preset-{}", p.id()))),
            None,
        ),
        Setting::RenderDistance => (
            g.render_distance.to_string(),
            gauge(g.render_distance, 2, 32),
        ),
        Setting::VerticalDistance => (
            g.vertical_distance.to_string(),
            gauge(g.vertical_distance, 2, 16),
        ),
        Setting::RenderScale => (
            format!("{} %", g.render_scale),
            gauge(g.render_scale, 50, 200),
        ),
        Setting::FpsLimit if g.fps_limit == 0 => (shared.tr("option-unlimited"), None),
        Setting::FpsLimit => (g.fps_limit.to_string(), None),
        Setting::Vsync => (on_off(shared, g.vsync), None),
        Setting::Fov => (format!("{}°", g.fov), gauge(g.fov, 50, 110)),
        Setting::AmbientOcclusion => (on_off(shared, g.ambient_occlusion), None),
        Setting::Fog => (on_off(shared, g.fog), None),
        Setting::Mipmaps => (on_off(shared, g.mipmaps), None),
        Setting::Anisotropy if g.anisotropy <= 1 => (shared.tr("ui-off"), None),
        Setting::Anisotropy => (format!("{}x", g.anisotropy), None),
        Setting::MeshThreads => (threads_label(shared, g.worker_threads), None),
        Setting::WorldThreads => (threads_label(shared, g.generation_threads), None),
        Setting::Language => (shared.tr("language-name"), None),
        Setting::Scale if i.scale == 0 => (shared.tr("ui-scale-auto"), None),
        Setting::Scale => (
            shared.tr_with("ui-scale-times", &[("scale", i.scale.to_string())]),
            None,
        ),
        Setting::Font => (
            shared.tr(match i.font {
                FontChoice::Pixel => "font-pixel",
                FontChoice::Readable => "font-readable",
            }),
            None,
        ),
        Setting::ColorFilter => (
            shared.tr(match i.color_filter {
                ColorFilter::None => "filter-none",
                ColorFilter::Protanopia => "filter-protanopia",
                ColorFilter::Deuteranopia => "filter-deuteranopia",
                ColorFilter::Tritanopia => "filter-tritanopia",
            }),
            None,
        ),
        Setting::ReduceCamera => (on_off(shared, i.reduce_camera_effects), None),
        Setting::PauseInMenus => (on_off(shared, i.pause_in_menus), None),
        Setting::BlockName => (on_off(shared, c.show_block_name), None),
        Setting::EditHud | Setting::ResetKeys => (String::new(), None),
        Setting::MouseSensitivity => (
            format!("{} %", c.mouse_sensitivity),
            gauge(c.mouse_sensitivity, 10, 300),
        ),
        Setting::StickSensitivity => (
            format!("{} %", c.stick_sensitivity),
            gauge(c.stick_sensitivity, 10, 300),
        ),
        Setting::InvertY => (on_off(shared, c.invert_y), None),
        Setting::DeadZone => (format!("{} %", c.dead_zone), gauge(c.dead_zone, 0, 50)),
        Setting::AutoJump => (on_off(shared, c.auto_jump), None),
        Setting::StepUp => (on_off(shared, c.step_up), None),
        Setting::CheckUpdates => (on_off(shared, game.check_updates), None),
        Setting::Autosave if game.autosave_minutes == 0 => (shared.tr("ui-off"), None),
        Setting::Autosave => (
            shared.tr_with(
                "ui-play-time-minutes",
                &[("minutes", game.autosave_minutes.to_string())],
            ),
            None,
        ),
        Setting::Backups => (
            game.backups_kept.to_string(),
            gauge(game.backups_kept, 0, 10),
        ),
        Setting::IdleUnfocused => (on_off(shared, game.idle_when_unfocused), None),
    }
}

fn step_setting(setting: Setting, s: &mut Settings, step: i32) {
    let graphics_changed = !matches!(setting, Setting::Preset) && GRAPHICS.contains(&setting);
    let cores = default_worker_threads() + 2;
    let (g, i, c, game) = (
        &mut s.graphics,
        &mut s.interface,
        &mut s.controls,
        &mut s.game,
    );
    match setting {
        Setting::Preset => {
            let next = cycle(&Preset::ALL, s.preset.unwrap_or(Preset::High), step);
            s.apply_preset(next);
        }
        Setting::RenderDistance => range(&mut g.render_distance, 2, 32, 1, step),
        Setting::VerticalDistance => range(&mut g.vertical_distance, 2, 16, 1, step),
        Setting::RenderScale => range(&mut g.render_scale, 50, 200, 10, step),
        Setting::FpsLimit => g.fps_limit = cycle(&FPS_LIMITS, g.fps_limit, step),
        Setting::Vsync => g.vsync = !g.vsync,
        Setting::Fov => range(&mut g.fov, 50, 110, 5, step),
        Setting::AmbientOcclusion => g.ambient_occlusion = !g.ambient_occlusion,
        Setting::Fog => g.fog = !g.fog,
        Setting::Mipmaps => g.mipmaps = !g.mipmaps,
        Setting::Anisotropy => g.anisotropy = cycle(&ANISOTROPY, g.anisotropy, step),
        Setting::MeshThreads => g.worker_threads = cycle_threads(g.worker_threads, cores, step),
        Setting::WorldThreads => {
            g.generation_threads = cycle_threads(g.generation_threads, cores, step)
        }
        Setting::Language => i.cycle_language(),
        Setting::Scale => range(&mut i.scale, 0, MAX_SCALE, 1, step),
        Setting::Font => {
            i.font = match i.font {
                FontChoice::Pixel => FontChoice::Readable,
                FontChoice::Readable => FontChoice::Pixel,
            }
        }
        Setting::ColorFilter => i.color_filter = cycle(&ColorFilter::ALL, i.color_filter, step),
        Setting::ReduceCamera => i.reduce_camera_effects = !i.reduce_camera_effects,
        Setting::PauseInMenus => i.pause_in_menus = !i.pause_in_menus,
        Setting::BlockName => c.show_block_name = !c.show_block_name,
        Setting::MouseSensitivity => range(&mut c.mouse_sensitivity, 10, 300, 10, step),
        Setting::StickSensitivity => range(&mut c.stick_sensitivity, 10, 300, 10, step),
        Setting::InvertY => c.invert_y = !c.invert_y,
        Setting::DeadZone => range(&mut c.dead_zone, 0, 50, 5, step),
        Setting::AutoJump => c.auto_jump = !c.auto_jump,
        Setting::StepUp => c.step_up = !c.step_up,
        Setting::CheckUpdates => game.check_updates = !game.check_updates,
        Setting::Autosave => game.autosave_minutes = cycle(&AUTOSAVE, game.autosave_minutes, step),
        Setting::Backups => range(&mut game.backups_kept, 0, 10, 1, step),
        Setting::IdleUnfocused => game.idle_when_unfocused = !game.idle_when_unfocused,
        Setting::EditHud | Setting::ResetKeys => {}
    }
    if graphics_changed {
        s.preset = None;
    }
}

fn range(value: &mut u32, min: u32, max: u32, unit: u32, step: i32) {
    let next = *value as i64 + i64::from(step) * i64::from(unit);
    *value = next.clamp(i64::from(min), i64::from(max)) as u32;
}

/// The next (or previous) value of `choices`, wrapping; an unknown value restarts at the first.
fn cycle<T: Copy + PartialEq>(choices: &[T], current: T, step: i32) -> T {
    let len = choices.len() as i32;
    let index = choices
        .iter()
        .position(|c| *c == current)
        .map_or(0, |i| i as i32);
    choices[(index + step.signum()).rem_euclid(len) as usize]
}

/// Automatic, then 1 to the number of cores.
fn cycle_threads(current: usize, cores: usize, step: i32) -> usize {
    let choices: Vec<usize> = std::iter::once(AUTO_THREADS).chain(1..=cores).collect();
    cycle(&choices, current, step)
}

/// Every translation key the options screen may show, for the missing-translation test.
#[cfg(test)]
pub fn translation_keys() -> Vec<String> {
    let settings = GRAPHICS
        .iter()
        .chain(INTERFACE)
        .chain(CONTROLS)
        .chain(GAME)
        .chain(&[Setting::ResetKeys]);
    let mut keys: Vec<String> = settings
        .flat_map(|&s| [setting_key(s), format!("{}-help", setting_key(s))])
        .chain(REBINDABLE.iter().map(|&a| action_key(a)))
        .chain(TABS.iter().map(|k| (*k).to_owned()))
        .chain(Preset::ALL.iter().map(|p| format!("preset-{}", p.id())))
        .collect();
    keys.extend(
        [
            "preset-custom",
            "option-unlimited",
            "option-auto",
            "language-name",
            "font-pixel",
            "font-readable",
            "filter-none",
            "filter-protanopia",
            "filter-deuteranopia",
            "filter-tritanopia",
            "option-key-waiting",
            "option-key-help",
        ]
        .map(str::to_owned),
    );
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_kebab_case() {
        assert_eq!(
            setting_key(Setting::AmbientOcclusion),
            "option-ambient-occlusion"
        );
        assert_eq!(action_key(Action::Hotbar1), "action-hotbar-1");
    }

    #[test]
    fn changing_a_graphics_value_leaves_the_preset() {
        let mut settings = Settings::default();
        step_setting(Setting::Fov, &mut settings, 1);
        assert_eq!((settings.graphics.fov, settings.preset), (75, None));
        step_setting(Setting::Preset, &mut settings, 1);
        assert!(settings.preset.is_some());
        step_setting(Setting::RenderDistance, &mut settings, -100);
        assert_eq!(settings.graphics.render_distance, 2);
    }
}
