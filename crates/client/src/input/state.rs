use super::{Action, Binding, Bindings};
use glam::Vec2;
use std::collections::HashSet;

/// Last device used, so on-screen prompts show keys or gamepad buttons accordingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDevice {
    KeyboardMouse,
    Gamepad,
    Touch,
}

/// Input collected during one frame, queried by the game through actions.
pub struct InputState {
    bindings: Bindings,
    pressed: HashSet<Binding>,
    just_pressed: [bool; Action::COUNT],
    mouse_delta: Vec2,
    /// Wheel notches since the last frame (positive = away from the player).
    scroll: f32,
    /// Text typed since the last frame (search fields).
    typed: String,
    /// Mouse position in window pixels.
    cursor: Option<(f64, f64)>,
    cursor_moved: bool,
    left_stick: Vec2,
    right_stick: Vec2,
    last_device: InputDevice,
    /// First input pressed this frame (to bind it in the options).
    fresh: Option<Binding>,
    /// Stick movements below this length are ignored.
    dead_zone: f32,
}

impl InputState {
    pub fn new(bindings: Bindings) -> Self {
        Self {
            bindings,
            pressed: HashSet::new(),
            just_pressed: [false; Action::COUNT],
            mouse_delta: Vec2::ZERO,
            scroll: 0.0,
            typed: String::new(),
            cursor: None,
            cursor_moved: false,
            left_stick: Vec2::ZERO,
            right_stick: Vec2::ZERO,
            last_device: InputDevice::KeyboardMouse,
            fresh: None,
            dead_zone: 0.15,
        }
    }

    pub fn set_binding(&mut self, binding: Binding, pressed: bool) {
        self.note_device(match binding {
            Binding::Pad(_) => InputDevice::Gamepad,
            Binding::Key(_) | Binding::Mouse(_) => InputDevice::KeyboardMouse,
        });
        if !pressed {
            self.pressed.remove(&binding);
            return;
        }
        if self.pressed.insert(binding) {
            self.fresh = self.fresh.or(Some(binding));
            for action in self.bindings.actions_for(binding) {
                self.just_pressed[action.index()] = true;
            }
        }
    }

    pub fn add_mouse_motion(&mut self, delta: Vec2) {
        self.note_device(InputDevice::KeyboardMouse);
        self.mouse_delta += delta;
    }

    pub fn add_scroll(&mut self, notches: f32) {
        self.note_device(InputDevice::KeyboardMouse);
        self.scroll += notches;
    }

    pub fn set_cursor(&mut self, position: (f64, f64)) {
        self.note_device(InputDevice::KeyboardMouse);
        self.cursor = Some(position);
        self.cursor_moved = true;
    }

    pub fn cursor(&self) -> Option<(f64, f64)> {
        self.cursor
    }

    /// The mouse moved this frame (menus then follow it rather than the keyboard focus).
    pub fn cursor_moved(&self) -> bool {
        self.cursor_moved
    }

    /// Control characters (Backspace arrives as `\u{8}` on Windows) are actions, not text.
    pub fn add_text(&mut self, text: &str) {
        self.typed.extend(text.chars().filter(|c| !c.is_control()));
    }

    /// Whole wheel notches since the last frame.
    pub fn scroll_steps(&self) -> i32 {
        self.scroll.round() as i32
    }

    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// Stick values in `[-1, 1]`, y up.
    pub fn set_sticks(&mut self, left: Vec2, right: Vec2) {
        let (left, right) = (
            dead_zone(left, self.dead_zone),
            dead_zone(right, self.dead_zone),
        );
        if left != Vec2::ZERO || right != Vec2::ZERO {
            self.note_device(InputDevice::Gamepad);
        }
        self.left_stick = left;
        self.right_stick = right;
    }

    pub fn note_touch(&mut self) {
        self.note_device(InputDevice::Touch);
    }

    pub fn is_held(&self, action: Action) -> bool {
        self.bindings
            .bindings_for(action)
            .any(|binding| self.pressed.contains(&binding))
    }

    pub fn just_pressed(&self, action: Action) -> bool {
        self.just_pressed[action.index()]
    }

    /// Movement wish: x = right, y = forward, length at most 1.
    pub fn movement(&self) -> Vec2 {
        let axis = |positive, negative| {
            f32::from(u8::from(self.is_held(positive)))
                - f32::from(u8::from(self.is_held(negative)))
        };
        let keys = Vec2::new(
            axis(Action::MoveRight, Action::MoveLeft),
            axis(Action::MoveForward, Action::MoveBackward),
        );
        (keys + self.left_stick).clamp_length_max(1.0)
    }

    /// Mouse movement in pixels since the last frame.
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    pub fn look_stick(&self) -> Vec2 {
        self.right_stick
    }

    pub fn bindings(&self) -> &Bindings {
        &self.bindings
    }

    /// New bindings or dead zone from the options.
    pub fn configure(&mut self, bindings: &Bindings, dead_zone: f32) {
        if self.bindings != *bindings {
            self.bindings = bindings.clone();
            self.release_all();
        }
        self.dead_zone = dead_zone.clamp(0.0, 0.9);
    }

    /// The first key, button or gamepad button pressed this frame.
    pub fn fresh_binding(&self) -> Option<Binding> {
        self.fresh
    }

    pub fn last_device(&self) -> InputDevice {
        self.last_device
    }

    /// Clears per-frame data; held buttons and stick positions persist.
    pub fn end_frame(&mut self) {
        self.just_pressed = [false; Action::COUNT];
        self.fresh = None;
        self.cursor_moved = false;
        self.mouse_delta = Vec2::ZERO;
        self.scroll = 0.0;
        self.typed.clear();
    }

    /// Forgets held inputs, e.g. when the window loses focus and releases would be missed.
    pub fn release_all(&mut self) {
        self.pressed.clear();
        self.left_stick = Vec2::ZERO;
        self.right_stick = Vec2::ZERO;
    }

    fn note_device(&mut self, device: InputDevice) {
        if self.last_device != device {
            tracing::debug!(?device, "input device changed");
            self.last_device = device;
        }
    }
}

/// Radial dead zone, rescaled so the output still covers the full range.
fn dead_zone(stick: Vec2, zone: f32) -> Vec2 {
    let length = stick.length();
    if length <= zone {
        return Vec2::ZERO;
    }
    let scaled = ((length - zone) / (1.0 - zone)).min(1.0);
    stick / length * scaled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::PadButton;
    use winit::keyboard::KeyCode;

    #[test]
    fn keyboard_and_gamepad_drive_the_same_action() {
        let mut input = InputState::new(Bindings::default());
        input.set_binding(Binding::Key(KeyCode::Space), true);
        input.set_binding(Binding::Pad(PadButton::South), true);
        input.set_binding(Binding::Key(KeyCode::Space), false);
        assert!(input.is_held(Action::Jump));
        assert_eq!(input.last_device(), InputDevice::KeyboardMouse);
        input.set_binding(Binding::Pad(PadButton::South), false);
        assert!(!input.is_held(Action::Jump));
    }

    #[test]
    fn just_pressed_lasts_one_frame() {
        let mut input = InputState::new(Bindings::default());
        input.set_binding(Binding::Key(KeyCode::Escape), true);
        assert!(input.just_pressed(Action::Pause));
        input.end_frame();
        input.set_binding(Binding::Key(KeyCode::Escape), true);
        assert!(!input.just_pressed(Action::Pause));
    }

    #[test]
    fn diagonal_movement_is_not_faster() {
        let mut input = InputState::new(Bindings::default());
        input.set_binding(Binding::Key(KeyCode::KeyW), true);
        input.set_binding(Binding::Key(KeyCode::KeyD), true);
        assert!((input.movement().length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn small_stick_drift_is_ignored() {
        let mut input = InputState::new(Bindings::default());
        input.set_sticks(Vec2::new(0.1, 0.05), Vec2::ZERO);
        assert_eq!(input.movement(), Vec2::ZERO);
        assert_eq!(input.last_device(), InputDevice::KeyboardMouse);
    }
}
