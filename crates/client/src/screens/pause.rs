//! Pause menu and simple dialogs.

use super::menu::{Effect, Menu, Shared, UiAction};
use crate::ui::ScreenModel;

pub struct PauseModel<'a> {
    pub shared: Shared<'a>,
}

impl ScreenModel for PauseModel<'_> {
    fn activate(&mut self, id: &str, _: usize) {
        let effect = match id {
            "resume" => Effect::CloseAll,
            "options" => Effect::Push(Menu::Options(Default::default())),
            "save_quit" => Effect::Action(UiAction::SaveAndQuit),
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

pub struct CrashModel<'a> {
    pub shared: Shared<'a>,
}

impl ScreenModel for CrashModel<'_> {
    fn activate(&mut self, id: &str, _: usize) {
        let report = match id {
            "report" => true,
            "ignore" => false,
            _ => return,
        };
        self.shared
            .effects
            .extend([Effect::Action(UiAction::SettleCrash(report)), Effect::Pop]);
    }
}
