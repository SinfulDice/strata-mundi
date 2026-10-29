//! The options screen: one tab per family of settings, one row per setting, with a short
//! description of what it does to performance. Changes apply at once.

mod packs;
mod values;

use super::menu::{Effect, Menu, Shared};
use crate::input::{Action, Binding};
use crate::settings::Settings;
use crate::ui::{OptionRow, ScreenModel};
use values::{
    CONTROLS, GAME, GRAPHICS, INTERFACE, Setting, kebab_key, setting_key, step_setting, value,
};

/// Rows visible at once (matches `options.ron`).
pub const VISIBLE_ROWS: usize = 11;

/// Tab titles; the keys tab lists `REBINDABLE` actions, the packs tab the resource packs.
const TABS: [&str; 6] = [
    "option-tab-graphics",
    "option-tab-interface",
    "option-tab-controls",
    "option-tab-keys",
    "option-tab-game",
    "option-tab-packs",
];
const KEYS_TAB: usize = 3;
const PACKS_TAB: usize = 5;

/// Actions the player can rebind (menu navigation keeps fixed keys so menus always work).
pub const REBINDABLE: [Action; 27] = [
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
    Action::ReloadResources,
    Action::OpenConsole,
];

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
    Pack(String),
    /// The resource packs folder is empty.
    NoPacks,
}

impl OptionsModel<'_> {
    fn rows(&self) -> Vec<Row> {
        match self.state.tab {
            KEYS_TAB => REBINDABLE
                .into_iter()
                .map(Row::Key)
                .chain(std::iter::once(Row::Setting(Setting::ResetKeys)))
                .collect(),
            PACKS_TAB if self.shared.packs.is_empty() => vec![Row::NoPacks],
            PACKS_TAB => self.shared.packs.iter().cloned().map(Row::Pack).collect(),
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
        match row {
            Row::Pack(name) => {
                let enabled = &self.shared.settings.game.resource_packs;
                let value = match packs::rank(enabled, name) {
                    Some(rank) => self
                        .shared
                        .tr_with("option-pack-rank", &[("rank", rank.to_string())]),
                    None => self.shared.tr("ui-off"),
                };
                OptionRow {
                    label: name.clone(),
                    value,
                    ..OptionRow::default()
                }
            }
            Row::NoPacks => OptionRow {
                label: self.shared.tr("option-packs-none"),
                ..OptionRow::default()
            },
            &Row::Key(action) => {
                let (keyboard, pad) = (self.key_labels)(action);
                OptionRow {
                    label: self.shared.tr(&action_key(action)),
                    value: format!("{keyboard} | {pad}"),
                    gauge: None,
                    highlight: self.state.capturing == Some(action),
                }
            }
            &Row::Setting(setting) => {
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
            Row::Pack(_) => self.shared.tr("option-pack-help"),
            Row::NoPacks => self.shared.tr("option-packs-none-help"),
        })
    }

    fn scroll(&mut self, _: &str, rows: i32) {
        let max = self.rows().len().saturating_sub(VISIBLE_ROWS) as i32;
        self.state.scroll = (self.state.scroll as i32 + rows).clamp(0, max) as usize;
    }

    fn adjust(&mut self, _: &str, index: usize, step: i32, _: Option<f32>) {
        match self.row_at(index) {
            Some(Row::Setting(setting))
                if !matches!(setting, Setting::EditHud | Setting::ResetKeys) =>
            {
                self.change(setting, step);
            }
            Some(Row::Pack(name)) => {
                packs::reorder(&mut self.shared.settings.game.resource_packs, &name, step);
                self.shared.effects.push(Effect::SettingsChanged);
            }
            _ => {}
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
                Some(Row::Pack(name)) => {
                    packs::toggle(&mut self.shared.settings.game.resource_packs, &name);
                    self.shared.effects.push(Effect::SettingsChanged);
                }
                Some(Row::NoPacks) | None => {}
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

pub fn action_key(action: Action) -> String {
    kebab_key("action", &format!("{action:?}"))
}

/// Settings by tab (tab title key, setting keys; each has a `-help` key), for the wiki.
pub fn catalog() -> Vec<(&'static str, Vec<String>)> {
    [
        (TABS[0], GRAPHICS),
        (TABS[1], INTERFACE),
        (TABS[2], CONTROLS),
        (TABS[4], GAME),
    ]
    .into_iter()
    .map(|(tab, settings)| (tab, settings.iter().map(|&s| setting_key(s)).collect()))
    .collect()
}

/// Every translation key the options screen may show, for the missing-translation test.
#[cfg(test)]
pub fn translation_keys() -> Vec<String> {
    use crate::settings::Preset;
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
            "option-pack-rank",
            "option-pack-help",
            "option-packs-none",
            "option-packs-none-help",
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
}
