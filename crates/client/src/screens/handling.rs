//! Menu input: focus (mouse, arrows, d-pad), activation, typing, the on-screen
//! keyboard and moving HUD elements in the editor.

use super::console::ConsoleState;
use super::keyboard::Keyboard;
use super::menu::{Effect, Menu};
use super::options::bind_captured;
use super::{Screens, UiAction};
use crate::gameplay::Gameplay;
use crate::input::{Action, Binding, InputDevice, InputState};
use crate::ui::{Direction, Hit, HitKind, navigate};
use strata_core::content::Content;

/// Result of one frame of menu input.
pub struct Handled {
    /// The input was meant for a menu (the game must ignore it).
    pub used: bool,
    pub actions: Vec<UiAction>,
}

/// The clipboard, after the effects: a copy done (or not), a field waiting for its text.
enum Clip {
    Copied(bool),
    Paste(String),
}

/// Widgets showing a window over longer content (world list, item grid, settings rows).
const SCROLLERS: [&str; 5] = ["list", "items", "options", "layers", "biomes"];

pub(super) const MOVES: [(Action, Direction); 4] = [
    (Action::MenuUp, Direction::Up),
    (Action::MenuDown, Direction::Down),
    (Action::MenuLeft, Direction::Left),
    (Action::MenuRight, Direction::Right),
];

/// During a cinematic only skipping is possible (if the shot allows it).
fn skip_cinematic(input: &InputState, game: &mut Gameplay) -> Handled {
    let skip = input.just_pressed(Action::Pause) || input.just_pressed(Action::MenuBack);
    if skip && game.cinematic.as_ref().is_some_and(|c| c.skippable()) {
        game.stop_cinematic();
    }
    Handled {
        used: true,
        actions: Vec::new(),
    }
}

impl Screens {
    /// Opens, drives and closes menus. In a world without a menu, only the inventory
    /// and pause keys are looked at.
    pub fn handle(
        &mut self,
        input: &InputState,
        mut game: Option<&mut Gameplay>,
        content: &Content,
        frame_seconds: f32,
    ) -> Handled {
        self.device = input.last_device();
        let steps = self.held_steps(input, frame_seconds);
        self.refresh_labels(content);
        if self.in_world && self.stack.is_empty() {
            if let Some(game) = game.as_deref_mut().filter(|g| g.cinematic.is_some()) {
                return skip_cinematic(input, game);
            }
            let menu = if input.just_pressed(Action::Pause) {
                Menu::Pause
            } else if input.just_pressed(Action::Inventory) {
                Menu::Inventory(Default::default())
            } else if input.just_pressed(Action::OpenConsole) {
                Menu::Console(ConsoleState::new())
            } else {
                return Handled {
                    used: false,
                    actions: Vec::new(),
                };
            };
            self.effects.push(Effect::Push(menu));
        } else if matches!(self.stack.last(), Some(Menu::Console(_))) {
            self.handle_console(input, game.as_deref_mut(), content);
        } else if self.keyboard.is_some() {
            self.handle_keyboard(input, game.as_deref_mut(), content);
        } else if self.editing.is_some() {
            if self.handle_typing(input, game.as_deref_mut(), content) {
                self.handle_menu(input, game.as_deref_mut(), content, steps);
            }
        } else {
            self.handle_menu(input, game.as_deref_mut(), content, steps);
        }
        let (actions, clip) = self.apply_effects();
        match clip {
            Some(Clip::Copied(copied)) => {
                self.with_model(game, content, |m| m.copied(copied));
            }
            Some(Clip::Paste(field)) => {
                let text = self.paste_text();
                self.with_model(game, content, |m| m.paste(&field, text.as_deref()));
            }
            None => {}
        }
        // Written once the change is over, not at every frame of a drag.
        if self.settings_dirty && !input.is_held(Action::Attack) {
            self.settings.save(&self.settings_path);
            self.settings_dirty = false;
        }
        Handled {
            used: true,
            actions,
        }
    }

    /// `steps`: ← → for the focused value this frame (taps and held repeats).
    fn handle_menu(
        &mut self,
        input: &InputState,
        mut game: Option<&mut Gameplay>,
        content: &Content,
        steps: i32,
    ) {
        if self.capture_binding(input) {
            return;
        }
        let cursor = self.cursor(input);
        self.follow_mouse(input, cursor);
        if self.handle_editor(input, cursor) {
            return;
        }
        let closes_inventory = input.just_pressed(Action::Inventory)
            && matches!(self.stack.last(), Some(Menu::Inventory(_)));
        if input.just_pressed(Action::MenuBack) || closes_inventory {
            let handled = input.just_pressed(Action::MenuBack)
                && self
                    .with_model(game.as_deref_mut(), content, |m| m.back())
                    .unwrap_or(false);
            if !handled && !matches!(self.stack.last(), Some(Menu::Title)) {
                self.effects.push(Effect::Pop);
            }
            return;
        }
        let focus = self.focus.clone();
        let slider = focus
            .clone()
            .filter(|f| matches!(f.kind, HitKind::Slider | HitKind::Option));
        match (&slider, steps) {
            (Some(slider), step) if step != 0 => {
                self.with_model(game.as_deref_mut(), content, |m| {
                    m.adjust(&slider.id, slider.index, step, None)
                });
            }
            _ => self.navigate_or_scroll(input, game.as_deref_mut(), content),
        }
        self.scroll_and_tabs(input, cursor, game.as_deref_mut(), content);
        self.drag_value(input, cursor, game.as_deref_mut(), content);
        let clicked = input
            .just_pressed(Action::Attack)
            .then(|| cursor.and_then(|at| Some((at, self.hit_at(at.0, at.1)?))))
            .flatten();
        if let Some((at, hit)) = &clicked {
            self.focus = Some(hit.clone());
            if self.click_value(game.as_deref_mut(), content, hit, *at) {
                return;
            }
        }
        let clicked = clicked.map(|(_, hit)| hit);
        let target = clicked.or(focus.filter(|_| input.just_pressed(Action::MenuConfirm)));
        if let Some(hit) = target {
            self.activate(input, game.as_deref_mut(), content, &hit);
        }
        if let Some(focus) = self.focus.clone() {
            for (slot, action) in Action::HOTBAR.into_iter().enumerate() {
                if input.just_pressed(action) {
                    self.with_model(game.as_deref_mut(), content, |m| {
                        m.quick_slot(&focus.id, focus.index, slot)
                    });
                }
            }
        }
    }

    /// Options, keys tab: the next input pressed becomes the action's binding (Escape
    /// cancels). Returns whether the input was taken.
    fn capture_binding(&mut self, input: &InputState) -> bool {
        let Some(Menu::Options(state)) = self.stack.last_mut() else {
            return false;
        };
        if state.capturing.is_none() {
            return false;
        }
        // Back (Escape, B) cancels; the pause input stays reserved so the menus can always
        // be reached again, even with a gamepad only.
        let reserved = |binding: Binding, action: Action| {
            input.bindings().actions_for(binding).any(|a| a == action)
        };
        match input.fresh_binding() {
            Some(binding) if reserved(binding, Action::MenuBack) => state.capturing = None,
            Some(binding) if reserved(binding, Action::Pause) => {}
            Some(binding) => {
                bind_captured(state, &mut self.settings, binding);
                self.effects.push(Effect::SettingsChanged);
            }
            None => {}
        }
        true
    }

    fn activate(
        &mut self,
        input: &InputState,
        game: Option<&mut Gameplay>,
        content: &Content,
        hit: &Hit,
    ) {
        self.focus = Some(hit.clone());
        match hit.kind {
            HitKind::TextField if input.last_device() == InputDevice::Gamepad => {
                self.keyboard = Some(Keyboard::new(hit.id.clone()));
                self.focus = None;
            }
            HitKind::TextField => self.editing = Some(hit.id.clone()),
            HitKind::Slider => {
                self.with_model(game, content, |m| m.adjust(&hit.id, hit.index, 1, None));
            }
            _ => {
                self.with_model(game, content, |m| m.activate(&hit.id, hit.index));
            }
        }
    }

    fn scroll_and_tabs(
        &mut self,
        input: &InputState,
        cursor: Option<(i32, i32)>,
        mut game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        let steps = input.scroll_steps();
        if steps != 0 && !self.wheel_value(game.as_deref_mut(), content, cursor, steps) {
            let id = self
                .focus
                .as_ref()
                .map(|f| f.id.clone())
                .unwrap_or_default();
            self.with_model(game.as_deref_mut(), content, |m| m.scroll(&id, -steps));
        }
        let tabs = i32::from(input.just_pressed(Action::HotbarNext))
            - i32::from(input.just_pressed(Action::HotbarPrevious));
        if tabs != 0 {
            self.with_model(game, content, |m| {
                let count = m.tabs("tabs").len() as i32;
                if count > 0 {
                    let current = m.selected("tabs").unwrap_or(0) as i32;
                    m.activate("tabs", (current + tabs).rem_euclid(count) as usize);
                }
            });
        }
    }

    /// Returns `true` when a click left the field: the click is then handled as usual.
    fn handle_typing(
        &mut self,
        input: &InputState,
        game: Option<&mut Gameplay>,
        content: &Content,
    ) -> bool {
        let Some(field) = self.editing.clone() else {
            return false;
        };
        let mut text = input.typed().to_owned();
        if input.just_pressed(Action::Erase) {
            text.push('\u{8}');
        }
        if !text.is_empty() {
            self.with_model(game, content, |m| m.type_text(&field, &text));
        }
        let cursor = self.cursor(input);
        let clicked_elsewhere = input.just_pressed(Action::Attack)
            && cursor
                .and_then(|(x, y)| self.hit_at(x, y))
                .is_none_or(|hit| hit.id != field);
        if input.just_pressed(Action::MenuConfirm)
            || input.just_pressed(Action::MenuBack)
            || clicked_elsewhere
        {
            self.editing = None;
        }
        clicked_elsewhere
    }

    /// Moves the focus; at the edge of a list or grid, scrolls it instead (gamepads and
    /// keyboards reach every row, not only the visible ones).
    fn navigate_or_scroll(
        &mut self,
        input: &InputState,
        mut game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        for (action, direction) in MOVES {
            if !input.just_pressed(action) {
                continue;
            }
            let next = navigate(&self.hits, self.focus.as_ref(), direction);
            let scroller = self
                .focus
                .clone()
                .filter(|f| SCROLLERS.contains(&f.id.as_str()));
            let rows = match direction {
                Direction::Up => -1,
                Direction::Down => 1,
                _ => 0,
            };
            match (scroller, next) {
                (Some(focus), next)
                    if rows != 0 && next.as_ref().is_none_or(|n| n.id != focus.id) =>
                {
                    let before =
                        self.with_model(game.as_deref_mut(), content, |m| m.rows(&focus.id));
                    self.with_model(game.as_deref_mut(), content, |m| m.scroll(&focus.id, rows));
                    let after =
                        self.with_model(game.as_deref_mut(), content, |m| m.rows(&focus.id));
                    // Nothing left to scroll: the focus may leave the list.
                    if before == after
                        && let Some(next) = next
                    {
                        self.focus = Some(next);
                    }
                }
                (_, next) => self.focus = next.or(self.focus.take()),
            }
        }
    }

    /// Cursor in interface pixels.
    pub(super) fn cursor(&self, input: &InputState) -> Option<(i32, i32)> {
        let scale = f64::from(self.scale.max(1));
        input
            .cursor()
            .map(|(x, y)| ((x / scale) as i32, (y / scale) as i32))
    }

    fn follow_mouse(&mut self, input: &InputState, cursor: Option<(i32, i32)>) {
        if let Some((x, y)) = cursor.filter(|_| input.cursor_moved()) {
            self.focus = self.hit_at(x, y);
        }
    }

    pub(super) fn hit_at(&self, x: i32, y: i32) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.rect.contains(x, y))
            .cloned()
    }

    /// The session's actions, and what the clipboard did or is asked for.
    fn apply_effects(&mut self) -> (Vec<UiAction>, Option<Clip>) {
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
