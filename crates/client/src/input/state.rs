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

const STICK_DEAD_ZONE: f32 = 0.15;

/// Input collected during one frame, queried by the game through actions.
pub struct InputState {
    bindings: Bindings,
    pressed: HashSet<Binding>,
    just_pressed: [bool; Action::COUNT],
    mouse_delta: Vec2,
    left_stick: Vec2,
    right_stick: Vec2,
    last_device: InputDevice,
}

impl InputState {
    pub fn new(bindings: Bindings) -> Self {
        Self {
            bindings,
            pressed: HashSet::new(),
            just_pressed: [false; Action::COUNT],
            mouse_delta: Vec2::ZERO,
            left_stick: Vec2::ZERO,
            right_stick: Vec2::ZERO,
            last_device: InputDevice::KeyboardMouse,
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
            for action in self.bindings.actions_for(binding) {
                self.just_pressed[action.index()] = true;
            }
        }
    }

    pub fn add_mouse_motion(&mut self, delta: Vec2) {
        self.note_device(InputDevice::KeyboardMouse);
        self.mouse_delta += delta;
    }

    /// Stick values in `[-1, 1]`, y up.
    pub fn set_sticks(&mut self, left: Vec2, right: Vec2) {
        let (left, right) = (dead_zone(left), dead_zone(right));
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

    pub fn last_device(&self) -> InputDevice {
        self.last_device
    }

    /// Clears per-frame data; held buttons and stick positions persist.
    pub fn end_frame(&mut self) {
        self.just_pressed = [false; Action::COUNT];
        self.mouse_delta = Vec2::ZERO;
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
fn dead_zone(stick: Vec2) -> Vec2 {
    let length = stick.length();
    if length <= STICK_DEAD_ZONE {
        return Vec2::ZERO;
    }
    let scaled = ((length - STICK_DEAD_ZONE) / (1.0 - STICK_DEAD_ZONE)).min(1.0);
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
