//! HUD editor: pick an element of the in-game overlay and move it (arrows, d-pad or
//! mouse drag). Positions are offsets from each element's anchor, so they follow any
//! window size or ratio.

use super::Screens;
use super::handling::MOVES;
use super::menu::{Effect, Menu, Shared};
use crate::input::{Action, InputState};
use crate::settings::InterfaceSettings;
use crate::ui::{HitKind, ScreenDef, ScreenModel};

/// Pixels moved per arrow press.
pub const STEP: i32 = 1;

pub struct EditorModel<'a> {
    pub selected: &'a mut Option<String>,
    pub shared: Shared<'a>,
}

impl ScreenModel for EditorModel<'_> {
    fn activate(&mut self, id: &str, _: usize) {
        let effect = match id {
            "reset" => {
                self.shared.settings.interface.hud.clear();
                Effect::SettingsChanged
            }
            "done" => Effect::Pop,
            _ => {
                *self.selected = Some(id.to_owned());
                return;
            }
        };
        self.shared.effects.push(effect);
    }
}

/// Moves HUD element `id` by `delta`, starting from its default place in `hud`.
pub fn move_element(
    settings: &mut InterfaceSettings,
    hud: &ScreenDef,
    id: &str,
    delta: (i32, i32),
) {
    let Some(default) = hud.offset_of(id) else {
        return;
    };
    let at = settings.hud.entry(id.to_owned()).or_insert(default);
    at.0 += delta.0;
    at.1 += delta.1;
}

impl Screens {
    /// HUD editor: once an element is selected, arrows and mouse drags move it.
    /// Returns whether the input was used.
    pub(super) fn handle_editor(&mut self, input: &InputState, cursor: Option<(i32, i32)>) -> bool {
        let Some(Menu::HudEditor { selected }) = self.stack.last_mut() else {
            return false;
        };
        let clicked = input
            .just_pressed(Action::Attack)
            .then_some(cursor)
            .flatten();
        if let Some((x, y)) = clicked
            && let Some(hit) = self.hits.iter().find(|h| h.rect.contains(x, y))
            && hit.kind == HitKind::Element
        {
            *selected = Some(hit.id.clone());
            self.dragging = Some(hit.clone());
            self.focus = Some(hit.clone());
        }
        let Some(id) = selected.clone() else {
            return false;
        };
        if input.just_pressed(Action::MenuBack) || input.just_pressed(Action::MenuConfirm) {
            *selected = None;
            return true;
        }
        let speed = if input.is_held(Action::Sprint) {
            8 * STEP
        } else {
            STEP
        };
        let mut delta = (0, 0);
        for (action, direction) in MOVES {
            if input.just_pressed(action) {
                let (dx, dy) = direction.offset();
                delta = (delta.0 + dx * speed, delta.1 + dy * speed);
            }
        }
        if input.is_held(Action::Attack)
            && self.dragging.as_ref().is_some_and(|d| d.id == id)
            && let (Some(now), Some(before)) = (cursor, self.last_cursor)
        {
            delta = (delta.0 + now.0 - before.0, delta.1 + now.1 - before.1);
        }
        if delta != (0, 0) {
            move_element(
                &mut self.settings.interface,
                &self.assets.screens.hud,
                &id,
                delta,
            );
            self.effects.push(Effect::SettingsChanged);
        }
        self.last_cursor = cursor;
        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn moving_starts_from_the_default_place() {
        let hud: ScreenDef = ron::from_str(
            "Screen(root: [Crosshair(id: \"crosshair\", anchor: Center, at: (2, 3))])",
        )
        .unwrap();
        let mut settings = InterfaceSettings::default();
        move_element(&mut settings, &hud, "crosshair", (1, -1));
        move_element(&mut settings, &hud, "unknown", (1, 1));
        assert_eq!(settings.hud.get("crosshair"), Some(&(3, 2)));
        assert_eq!(settings.hud.len(), 1);
    }
}
