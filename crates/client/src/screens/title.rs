use super::menu::{Effect, Menu, Shared, UiAction};
use crate::ui::ScreenModel;

/// Main menu over the rotating panorama.
pub struct TitleModel<'a> {
    pub shared: Shared<'a>,
}

impl ScreenModel for TitleModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        match id {
            "version" => Some(self.shared.tr_with(
                "ui-version",
                &[("version", env!("CARGO_PKG_VERSION").into())],
            )),
            "news" => Some(
                self.shared
                    .newer_version
                    .map_or_else(String::new, |version| {
                        self.shared
                            .tr_with("ui-update-available", &[("version", version.to_owned())])
                    }),
            ),
            _ => None,
        }
    }

    fn activate(&mut self, id: &str, _: usize) {
        let effect = match id {
            "play" => Effect::Action(UiAction::ShowWorlds),
            "options" => Effect::Push(Menu::Options(Default::default())),
            "quit" => Effect::Action(UiAction::Quit),
            _ => return,
        };
        self.shared.effects.push(effect);
    }
}
