//! Names of keys and buttons for on-screen prompts (`<key:…>` markup).

use super::{Action, Binding, Bindings, InputDevice, PadButton};
use std::collections::HashMap;
use strata_core::lang::Translations;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// Letters as printed on the player's keyboard, learned from what each physical key
/// types (`KeyW` types "z" on AZERTY). Unknown keys fall back to their QWERTY letter.
#[derive(Debug, Default)]
pub struct KeyNames {
    learned: HashMap<KeyCode, String>,
}

impl KeyNames {
    pub fn learn(&mut self, code: KeyCode, text: &str) {
        let mut chars = text.chars();
        if let (Some(ch), None) = (chars.next(), chars.next())
            && !ch.is_control()
            && !ch.is_whitespace()
        {
            self.learned.insert(code, ch.to_uppercase().collect());
        }
    }

    /// Label of the first binding of `action` for the last device used.
    pub fn label(
        &self,
        bindings: &Bindings,
        device: InputDevice,
        action: Action,
        translations: &Translations,
        language: &str,
    ) -> String {
        let wants_pad = device == InputDevice::Gamepad;
        let binding = bindings
            .bindings_for(action)
            .find(|b| matches!(b, Binding::Pad(_)) == wants_pad)
            .or_else(|| bindings.bindings_for(action).next());
        let tr = |key: &str| translations.text(language, key);
        match binding {
            Some(Binding::Key(code)) => self
                .learned
                .get(&code)
                .cloned()
                .or_else(|| plain_key(code))
                .unwrap_or_else(|| tr(special_key(code))),
            Some(Binding::Mouse(button)) => tr(mouse_key(button)),
            Some(Binding::Pad(button)) => tr(pad_key(button)),
            None => tr("key-other"),
        }
    }
}

/// Letters, digits and function keys: their name is their label.
fn plain_key(code: KeyCode) -> Option<String> {
    let name = format!("{code:?}");
    let label = name
        .strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .or_else(|| {
            name.strip_prefix('F')
                .filter(|n| n.parse::<u8>().is_ok())
                .map(|_| name.as_str())
        })?;
    Some(label.to_owned())
}

fn special_key(code: KeyCode) -> &'static str {
    match code {
        KeyCode::Escape => "key-escape",
        KeyCode::Space => "key-space",
        KeyCode::Enter => "key-enter",
        KeyCode::Backspace => "key-backspace",
        KeyCode::Tab => "key-tab",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "key-shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "key-ctrl",
        KeyCode::AltLeft | KeyCode::AltRight => "key-alt",
        KeyCode::ArrowUp => "key-up",
        KeyCode::ArrowDown => "key-down",
        KeyCode::ArrowLeft => "key-left",
        KeyCode::ArrowRight => "key-right",
        _ => "key-other",
    }
}

fn mouse_key(button: MouseButton) -> &'static str {
    match button {
        MouseButton::Left => "key-mouse-left",
        MouseButton::Right => "key-mouse-right",
        MouseButton::Middle => "key-mouse-middle",
        _ => "key-mouse-other",
    }
}

/// Named after the New 3DS layout the DS copies (A right, B bottom, X top, Y left).
fn pad_key(button: PadButton) -> &'static str {
    match button {
        PadButton::South => "pad-south",
        PadButton::East => "pad-east",
        PadButton::North => "pad-north",
        PadButton::West => "pad-west",
        PadButton::L1 => "pad-l1",
        PadButton::R1 => "pad-r1",
        PadButton::L2 => "pad-l2",
        PadButton::R2 => "pad-r2",
        PadButton::Select => "pad-select",
        PadButton::Start => "pad-start",
        PadButton::LeftStick => "pad-left-stick",
        PadButton::RightStick => "pad-right-stick",
        PadButton::DPadUp => "pad-up",
        PadButton::DPadDown => "pad-down",
        PadButton::DPadLeft => "pad-left",
        PadButton::DPadRight => "pad-right",
    }
}

/// Every translation key a label may use, for the missing-translation test.
pub fn translation_keys() -> Vec<String> {
    let keys = [
        KeyCode::Escape,
        KeyCode::Space,
        KeyCode::Enter,
        KeyCode::Backspace,
        KeyCode::Tab,
        KeyCode::ShiftLeft,
        KeyCode::ControlLeft,
        KeyCode::AltLeft,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::F24,
    ]
    .map(special_key);
    let mouse = [
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
    ]
    .map(mouse_key);
    let pad = [
        PadButton::South,
        PadButton::East,
        PadButton::North,
        PadButton::West,
        PadButton::L1,
        PadButton::R1,
        PadButton::L2,
        PadButton::R2,
        PadButton::Select,
        PadButton::Start,
        PadButton::LeftStick,
        PadButton::RightStick,
        PadButton::DPadUp,
        PadButton::DPadDown,
        PadButton::DPadLeft,
        PadButton::DPadRight,
    ]
    .map(pad_key);
    keys.iter()
        .chain(&mouse)
        .chain(&pad)
        .map(|k| (*k).to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learned_letters_win_over_qwerty_names() {
        let mut names = KeyNames::default();
        let bindings = Bindings::default();
        let translations = Translations::default();
        let label = |names: &KeyNames, device| {
            names.label(&bindings, device, Action::MoveForward, &translations, "fr")
        };
        assert_eq!(label(&names, InputDevice::KeyboardMouse), "W");
        names.learn(KeyCode::KeyW, "z");
        assert_eq!(label(&names, InputDevice::KeyboardMouse), "Z");
        assert_eq!(label(&names, InputDevice::Gamepad), "pad-up");
        assert_eq!(plain_key(KeyCode::F7).as_deref(), Some("F7"));
        assert_eq!(plain_key(KeyCode::Escape), None);
    }
}
