//! What the menus asked for during a frame: menus pushed or popped, settings changed,
//! session actions, the clipboard.

use super::Screens;
use super::menu::{Effect, Menu, UiAction};

/// The clipboard, after the effects: a copy done (or not), a field waiting for its text.
pub(super) enum Clip {
    Copied(bool),
    Paste(String),
}

impl Screens {
    /// The session's actions, and what the clipboard did or is asked for.
    pub(super) fn apply_effects(&mut self) -> (Vec<UiAction>, Option<Clip>) {
        let (mut actions, mut clip) = (Vec::new(), None);
        for effect in std::mem::take(&mut self.effects) {
            match effect {
                Effect::Push(menu) => {
                    self.stack.push(menu);
                    self.reset_focus();
                }
                Effect::Pop => {
                    self.stack.pop();
                    if !self.in_world && self.stack.is_empty() {
                        self.stack.push(Menu::Title);
                    }
                    self.reset_focus();
                }
                Effect::CloseAll => {
                    self.stack.truncate(usize::from(!self.in_world));
                    self.reset_focus();
                }
                Effect::SettingsChanged => {
                    self.settings_dirty = true;
                    self.labels_stale = true;
                }
                Effect::Action(action) => actions.push(action),
                Effect::ClearFocus => {
                    self.focus = None;
                    self.dragging = None;
                }
                Effect::Copy(text) => clip = Some(Clip::Copied(self.copy_text(text))),
                Effect::Paste(field) => clip = Some(Clip::Paste(field)),
            }
        }
        (actions, clip)
    }
}
