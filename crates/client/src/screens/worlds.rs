//! World list and delete confirmation.

use super::creation::CreateForm;
use super::menu::{Effect, Menu, Shared, UiAction};
use crate::ui::{ListRow, ScreenModel};
use strata_protocol::{GameMode, WorldStatus, WorldSummary};

/// Rows of the list visible at once (matches `worlds.ron`).
pub const VISIBLE_ROWS: usize = 5;

pub struct WorldsModel<'a> {
    pub selected: &'a mut Option<usize>,
    pub scroll: &'a mut usize,
    pub shared: Shared<'a>,
}

impl WorldsModel<'_> {
    fn chosen(&self) -> Option<&WorldSummary> {
        self.selected.and_then(|i| self.shared.worlds.get(i))
    }

    fn row(&self, index: usize, world: &WorldSummary) -> ListRow {
        let detail = match world.status {
            WorldStatus::Ready => self.shared.tr_with(
                "ui-world-detail",
                &[
                    ("date", date(world.last_played)),
                    ("mode", self.shared.tr(mode_key(world.mode))),
                    ("time", play_time(&self.shared, world.play_seconds)),
                ],
            ),
            WorldStatus::TooOld => self.shared.tr("ui-world-too-old"),
            WorldStatus::TooNew => self.shared.tr("ui-world-too-new"),
            WorldStatus::Broken => self.shared.tr("ui-world-broken"),
        };
        ListRow {
            title: world.name.clone(),
            detail,
            icon: self.shared.icons.get(index).copied().flatten(),
            dim: world.status != WorldStatus::Ready,
        }
    }
}

impl ScreenModel for WorldsModel<'_> {
    fn rows(&self, _: &str) -> Vec<ListRow> {
        let worlds = self.shared.worlds.iter().enumerate();
        worlds
            .skip(*self.scroll)
            .take(VISIBLE_ROWS)
            .map(|(index, world)| self.row(index, world))
            .collect()
    }

    fn selected(&self, _: &str) -> Option<usize> {
        self.selected
            .and_then(|i| i.checked_sub(*self.scroll))
            .filter(|&i| i < VISIBLE_ROWS)
    }

    fn text(&self, id: &str) -> Option<String> {
        (id == "empty" && self.shared.worlds.is_empty()).then(|| self.shared.tr("ui-worlds-empty"))
    }

    fn scroll(&mut self, _: &str, rows: i32) {
        let max = self.shared.worlds.len().saturating_sub(VISIBLE_ROWS) as i32;
        *self.scroll = (*self.scroll as i32 + rows).clamp(0, max) as usize;
    }

    fn activate(&mut self, id: &str, index: usize) {
        let effect = match id {
            "list" => {
                let world = *self.scroll + index;
                if *self.selected == Some(world) {
                    self.open_selected();
                    return;
                }
                *self.selected = Some(world);
                return;
            }
            "play" => {
                self.open_selected();
                return;
            }
            "create" => Effect::Push(Menu::CreateWorld(Box::new(CreateForm::new(&self.shared)))),
            "delete" => match self.chosen() {
                Some(world) => Effect::Push(Menu::ConfirmDelete {
                    id: world.id.clone(),
                    name: world.name.clone(),
                }),
                None => return,
            },
            "back" => Effect::Pop,
            _ => return,
        };
        self.shared.effects.push(effect);
    }
}

impl WorldsModel<'_> {
    fn open_selected(&mut self) {
        let effect = match self.chosen() {
            Some(world) if world.status == WorldStatus::Ready => {
                Effect::Action(UiAction::Play(world.id.clone()))
            }
            Some(world) if world.status == WorldStatus::Broken => Effect::Push(Menu::Notice {
                key: "ui-error-broken",
                id: None,
            }),
            Some(_) => Effect::Push(Menu::Notice {
                key: "ui-error-incompatible",
                id: None,
            }),
            None => return,
        };
        self.shared.effects.push(effect);
    }
}

pub struct ConfirmModel<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub shared: Shared<'a>,
}

impl ScreenModel for ConfirmModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        (id == "question").then(|| {
            self.shared
                .tr_with("ui-delete-question", &[("name", self.name.to_owned())])
        })
    }

    fn activate(&mut self, id: &str, _: usize) {
        let effect = match id {
            "delete" => Effect::Action(UiAction::Delete(self.id.to_owned())),
            "cancel" => Effect::Pop,
            _ => return,
        };
        self.shared.effects.push(effect);
    }
}

/// Applies typed characters (`\u{8}` erases) within a length limit.
pub fn edit(field: &mut String, text: &str, max: usize, allowed: impl Fn(char) -> bool) {
    for ch in text.chars() {
        if ch == '\u{8}' {
            field.pop();
        } else if !ch.is_control() && allowed(ch) && field.chars().count() < max {
            field.push(ch);
        }
    }
}

pub fn mode_key(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Creative => "ui-mode-creative",
        GameMode::Survival => "ui-mode-survival",
    }
}

/// Local date and time of a save, written the same in every language: `2026-10-23 14:05`.
fn date(seconds: i64) -> String {
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(|utc| {
            utc.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

fn play_time(shared: &Shared<'_>, seconds: u64) -> String {
    let minutes = seconds / 60;
    let (hours, minutes) = (minutes / 60, format!("{:02}", minutes % 60));
    if hours > 0 {
        shared.tr_with(
            "ui-play-time-hours",
            &[("hours", hours.to_string()), ("minutes", minutes)],
        )
    } else {
        shared.tr_with(
            "ui-play-time-minutes",
            &[("minutes", (seconds / 60).to_string())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_respects_limits_and_filters() {
        let mut field = String::from("12");
        edit(&mut field, "3a4\u{8}5", 4, |c| c.is_ascii_digit());
        assert_eq!(field, "1235");
    }
}
