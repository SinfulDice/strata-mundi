//! Menu input: focus (mouse, arrows, d-pad), activation, typing, the on-screen
//! keyboard and moving HUD elements in the editor.

use super::editor::{STEP, move_element};
use super::keyboard::Keyboard;
use super::menu::{Effect, Menu};
use super::{Screens, UiAction};
use crate::gameplay::Gameplay;
use crate::input::{Action, InputDevice, InputState};
use crate::ui::{Direction, Hit, HitKind, navigate};
use strata_core::content::Content;

/// Result of one frame of menu input.
pub struct Handled {
    /// The input was meant for a menu (the game must ignore it).
    pub used: bool,
    pub actions: Vec<UiAction>,
}

/// Widgets showing a window over longer content (world list, item grid).
const SCROLLERS: [&str; 2] = ["list", "items"];

const MOVES: [(Action, Direction); 4] = [
    (Action::MenuUp, Direction::Up),
    (Action::MenuDown, Direction::Down),
    (Action::MenuLeft, Direction::Left),
    (Action::MenuRight, Direction::Right),
];

impl Screens {
    /// Opens, drives and closes menus. In a world without a menu, only the inventory
    /// and pause keys are looked at.
    pub fn handle(
        &mut self,
        input: &InputState,
        mut game: Option<&mut Gameplay>,
        content: &Content,
    ) -> Handled {
        if self.in_world && self.stack.is_empty() {
            let menu = if input.just_pressed(Action::Inventory) {
                Menu::Inventory(Default::default())
            } else if input.just_pressed(Action::Pause) {
                Menu::Pause
            } else {
                return Handled {
                    used: false,
                    actions: Vec::new(),
                };
            };
            self.effects.push(Effect::Push(menu));
        } else if self.keyboard.is_some() {
            self.handle_keyboard(input, game, content);
        } else if self.editing.is_some() {
            if self.handle_typing(input, game.as_deref_mut(), content) {
                self.handle_menu(input, game, content);
            }
        } else {
            self.handle_menu(input, game, content);
        }
        let actions = self.apply_effects();
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

    fn handle_menu(
        &mut self,
        input: &InputState,
        mut game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        let cursor = self.cursor(input);
        self.follow_mouse(input, cursor);
        if self.handle_editor(input, cursor) {
            return;
        }
        let closes_inventory = input.just_pressed(Action::Inventory)
            && matches!(self.stack.last(), Some(Menu::Inventory(_)));
        if input.just_pressed(Action::MenuBack) || closes_inventory {
            if !matches!(self.stack.last(), Some(Menu::Title)) {
                self.effects.push(Effect::Pop);
            }
            return;
        }
        let focus = self.focus.clone();
        let slider = focus.clone().filter(|f| f.kind == HitKind::Slider);
        let step = i32::from(input.just_pressed(Action::MenuRight))
            - i32::from(input.just_pressed(Action::MenuLeft));
        match (&slider, step) {
            (Some(slider), step) if step != 0 => {
                self.with_model(game.as_deref_mut(), content, |m| {
                    m.adjust(&slider.id, step, None)
                });
            }
            _ => self.navigate_or_scroll(input, game.as_deref_mut(), content),
        }
        self.scroll_and_tabs(input, game.as_deref_mut(), content);
        self.drag_slider(input, cursor, game.as_deref_mut(), content);
        let clicked = input
            .just_pressed(Action::Attack)
            .then(|| cursor.and_then(|(x, y)| self.hit_at(x, y)))
            .flatten();
        let target = clicked.or(focus.filter(|_| input.just_pressed(Action::MenuConfirm)));
        if let Some(hit) = target {
            let fraction = cursor.map(|(x, _)| (x - hit.rect.x) as f32 / hit.rect.w.max(1) as f32);
            self.activate(input, game.as_deref_mut(), content, &hit, fraction);
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

    fn activate(
        &mut self,
        input: &InputState,
        game: Option<&mut Gameplay>,
        content: &Content,
        hit: &Hit,
        fraction: Option<f32>,
    ) {
        self.focus = Some(hit.clone());
        match hit.kind {
            HitKind::TextField if input.last_device() == InputDevice::Gamepad => {
                self.keyboard = Some(Keyboard::new(hit.id.clone()));
                self.focus = None;
            }
            HitKind::TextField => self.editing = Some(hit.id.clone()),
            HitKind::Slider => {
                let by_mouse = input.just_pressed(Action::Attack);
                if by_mouse {
                    self.dragging = Some(hit.clone());
                }
                let fraction = fraction.filter(|_| by_mouse);
                self.with_model(game, content, |m| m.adjust(&hit.id, 1, fraction));
            }
            _ => {
                self.with_model(game, content, |m| m.activate(&hit.id, hit.index));
            }
        }
    }

    fn scroll_and_tabs(
        &mut self,
        input: &InputState,
        mut game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        let steps = input.scroll_steps();
        if steps != 0 {
            self.with_model(game.as_deref_mut(), content, |m| m.scroll("", -steps));
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

    fn drag_slider(
        &mut self,
        input: &InputState,
        cursor: Option<(i32, i32)>,
        game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        if !input.is_held(Action::Attack) {
            self.dragging = None;
            return;
        }
        let Some((drag, (x, _))) = self.dragging.clone().zip(cursor) else {
            return;
        };
        if drag.kind == HitKind::Slider && input.cursor_moved() {
            let fraction = (x - drag.rect.x) as f32 / drag.rect.w.max(1) as f32;
            self.with_model(game, content, |m| m.adjust(&drag.id, 0, Some(fraction)));
        }
    }

    /// HUD editor: once an element is selected, arrows and mouse drags move it.
    /// Returns whether the input was used.
    fn handle_editor(&mut self, input: &InputState, cursor: Option<(i32, i32)>) -> bool {
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
            move_element(&mut self.settings, &self.assets.screens.hud, &id, delta);
            self.effects.push(Effect::SettingsChanged);
        }
        self.last_cursor = cursor;
        true
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

    fn handle_keyboard(
        &mut self,
        input: &InputState,
        game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        if input.just_pressed(Action::MenuBack) {
            self.keyboard = None;
            return;
        }
        let cursor = self.cursor(input);
        if let Some((x, y)) = cursor.filter(|_| input.cursor_moved()) {
            self.focus = self
                .keyboard_hits
                .iter()
                .find(|h| h.rect.contains(x, y))
                .cloned();
        }
        for (action, direction) in MOVES {
            if input.just_pressed(action) {
                let next = navigate(&self.keyboard_hits, self.focus.as_ref(), direction);
                self.focus = next.or(self.focus.take());
            }
        }
        let pressed = input.just_pressed(Action::MenuConfirm) || input.just_pressed(Action::Attack);
        let Some(key) = self.focus.clone().filter(|_| pressed) else {
            return;
        };
        let Some((field, typed, done)) = self.press_key(content, key.index) else {
            return;
        };
        if !typed.is_empty() {
            self.with_model(game, content, |m| m.type_text(&field, &typed));
        }
        if done {
            self.keyboard = None;
            self.focus = None;
        }
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
    fn cursor(&self, input: &InputState) -> Option<(i32, i32)> {
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

    fn hit_at(&self, x: i32, y: i32) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.rect.contains(x, y))
            .cloned()
    }

    fn apply_effects(&mut self) -> Vec<UiAction> {
        let mut actions = Vec::new();
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
                Effect::SettingsChanged => self.settings_dirty = true,
                Effect::Action(action) => actions.push(action),
            }
        }
        actions
    }
}
