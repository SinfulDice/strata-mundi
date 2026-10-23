//! HUD editor: pick an element of the in-game overlay and move it (arrows, d-pad or
//! mouse drag). Positions are offsets from each element's anchor, so they follow any
//! window size or ratio.

use super::menu::{Effect, Shared};
use crate::settings::InterfaceSettings;
use crate::ui::{ScreenDef, ScreenModel};

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
