//! Pause menu, interface options and simple notices.

use super::menu::{Effect, Menu, Shared, UiAction};
use crate::interface_settings::MAX_SCALE;
use crate::ui::ScreenModel;

pub struct PauseModel<'a> {
    pub shared: Shared<'a>,
}

impl ScreenModel for PauseModel<'_> {
    fn activate(&mut self, id: &str, _: usize) {
        let effect = match id {
            "resume" => Effect::CloseAll,
            "options" => Effect::Push(Menu::Options),
            "save_quit" => Effect::Action(UiAction::SaveAndQuit),
            _ => return,
        };
        self.shared.effects.push(effect);
    }
}

pub struct OptionsModel<'a> {
    pub shared: Shared<'a>,
}

impl OptionsModel<'_> {
    fn yes_no(&self, value: bool) -> String {
        self.shared.tr(if value { "ui-on" } else { "ui-off" })
    }
}

impl ScreenModel for OptionsModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        let settings = &self.shared.settings;
        match id {
            "language" => Some(self.shared.tr("ui-option-language")),
            "pause" => Some(self.shared.tr_with(
                "ui-option-pause-menus",
                &[("value", self.yes_no(settings.pause_in_menus))],
            )),
            _ => None,
        }
    }

    fn slider(&self, id: &str) -> Option<(f32, String)> {
        (id == "scale").then(|| {
            let scale = self.shared.settings.scale;
            let value = if scale == 0 {
                self.shared.tr("ui-scale-auto")
            } else {
                format!("×{scale}")
            };
            let caption = self.shared.tr_with("ui-option-scale", &[("value", value)]);
            (scale as f32 / MAX_SCALE as f32, caption)
        })
    }

    fn adjust(&mut self, id: &str, step: i32, fraction: Option<f32>) {
        if id != "scale" {
            return;
        }
        let settings = &mut self.shared.settings;
        settings.scale = match fraction {
            Some(f) => (f.clamp(0.0, 1.0) * MAX_SCALE as f32).round() as u32,
            None => (settings.scale as i32 + step).clamp(0, MAX_SCALE as i32) as u32,
        };
        self.shared.effects.push(Effect::SettingsChanged);
    }

    fn activate(&mut self, id: &str, _: usize) {
        let effect = match id {
            "language" => {
                self.shared.settings.cycle_language();
                Effect::SettingsChanged
            }
            "pause" => {
                let settings = &mut self.shared.settings;
                settings.pause_in_menus = !settings.pause_in_menus;
                Effect::SettingsChanged
            }
            "hud" => Effect::Push(Menu::HudEditor { selected: None }),
            "back" => Effect::Pop,
            _ => return,
        };
        self.shared.effects.push(effect);
    }
}

/// A message with an OK button (a world that could not be opened…).
pub struct NoticeModel<'a> {
    pub key: &'a str,
    pub shared: Shared<'a>,
}

impl ScreenModel for NoticeModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        (id == "message").then(|| self.shared.tr(self.key))
    }

    fn activate(&mut self, id: &str, _: usize) {
        if id == "ok" {
            self.shared.effects.push(Effect::Pop);
        }
    }
}
