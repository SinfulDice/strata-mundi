use super::{Binding, InputState, PadButton};
use gilrs::{Axis, Button, EventType, Gilrs};
use glam::Vec2;

/// Polls every connected gamepad through `gilrs` and feeds the shared input state.
pub struct Gamepads {
    gilrs: Option<Gilrs>,
}

impl Gamepads {
    /// Gamepad support is optional: without it the game still runs on keyboard and mouse.
    pub fn connect() -> Self {
        let gilrs = Gilrs::new()
            .inspect_err(|error| tracing::warn!(%error, "gamepads unavailable"))
            .ok();
        Self { gilrs }
    }

    pub fn poll(&mut self, input: &mut InputState) {
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(button, _) => press(input, button, true),
                EventType::ButtonReleased(button, _) => press(input, button, false),
                EventType::Connected => tracing::info!(id = %event.id, "gamepad connected"),
                EventType::Disconnected => tracing::info!(id = %event.id, "gamepad disconnected"),
                _ => {}
            }
        }
        let (mut left, mut right) = (Vec2::ZERO, Vec2::ZERO);
        for (_, pad) in gilrs.gamepads() {
            left += Vec2::new(pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            right += Vec2::new(pad.value(Axis::RightStickX), pad.value(Axis::RightStickY));
        }
        input.set_sticks(left.clamp_length_max(1.0), right.clamp_length_max(1.0));
    }
}

fn press(input: &mut InputState, button: Button, pressed: bool) {
    if let Some(button) = pad_button(button) {
        input.set_binding(Binding::Pad(button), pressed);
    }
}

fn pad_button(button: Button) -> Option<PadButton> {
    Some(match button {
        Button::South => PadButton::South,
        Button::East => PadButton::East,
        Button::North => PadButton::North,
        Button::West => PadButton::West,
        Button::LeftTrigger => PadButton::L1,
        Button::RightTrigger => PadButton::R1,
        Button::LeftTrigger2 => PadButton::L2,
        Button::RightTrigger2 => PadButton::R2,
        Button::Select => PadButton::Select,
        Button::Start => PadButton::Start,
        Button::LeftThumb => PadButton::LeftStick,
        Button::RightThumb => PadButton::RightStick,
        Button::DPadUp => PadButton::DPadUp,
        Button::DPadDown => PadButton::DPadDown,
        Button::DPadLeft => PadButton::DPadLeft,
        Button::DPadRight => PadButton::DPadRight,
        _ => return None,
    })
}
