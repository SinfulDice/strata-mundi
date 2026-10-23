use super::Action;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// Gamepad buttons, laid out like a New 3DS (ABXY named by position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadButton {
    South,
    East,
    North,
    West,
    L1,
    R1,
    L2,
    R2,
    Select,
    Start,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// A physical input. Keys are physical codes: `KeyW` is the key labelled Z on AZERTY.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    Pad(PadButton),
}

/// Which inputs trigger which action. Remappable (options screen, milestone 8).
#[derive(Debug, Clone)]
pub struct Bindings {
    entries: Vec<(Binding, Action)>,
}

impl Bindings {
    pub fn actions_for(&self, binding: Binding) -> impl Iterator<Item = Action> + '_ {
        self.entries
            .iter()
            .filter(move |(b, _)| *b == binding)
            .map(|(_, action)| *action)
    }

    pub fn bindings_for(&self, action: Action) -> impl Iterator<Item = Binding> + '_ {
        self.entries
            .iter()
            .filter(move |(_, a)| *a == action)
            .map(|(binding, _)| *binding)
    }
}

impl Default for Bindings {
    fn default() -> Self {
        use Action::*;
        use Binding::{Key, Pad};
        Self {
            entries: vec![
                (Key(KeyCode::KeyW), MoveForward),
                (Key(KeyCode::KeyS), MoveBackward),
                (Key(KeyCode::KeyA), MoveLeft),
                (Key(KeyCode::KeyD), MoveRight),
                (Key(KeyCode::Space), Jump),
                (Key(KeyCode::ShiftLeft), Sneak),
                (Key(KeyCode::ControlLeft), Sprint),
                (Key(KeyCode::Escape), Pause),
                (Binding::Mouse(MouseButton::Left), Attack),
                (Binding::Mouse(MouseButton::Right), Use),
                (Binding::Mouse(MouseButton::Middle), PickBlock),
                (Key(KeyCode::KeyE), Inventory),
                (Key(KeyCode::Digit1), Hotbar1),
                (Key(KeyCode::Digit2), Hotbar2),
                (Key(KeyCode::Digit3), Hotbar3),
                (Key(KeyCode::Digit4), Hotbar4),
                (Key(KeyCode::Digit5), Hotbar5),
                (Key(KeyCode::Digit6), Hotbar6),
                (Key(KeyCode::Digit7), Hotbar7),
                (Key(KeyCode::Digit8), Hotbar8),
                (Key(KeyCode::Digit9), Hotbar9),
                (Key(KeyCode::F7), RenderDistanceDown),
                (Key(KeyCode::F8), RenderDistanceUp),
                (Pad(PadButton::DPadUp), MoveForward),
                (Pad(PadButton::DPadDown), MoveBackward),
                (Pad(PadButton::DPadLeft), MoveLeft),
                (Pad(PadButton::DPadRight), MoveRight),
                (Pad(PadButton::South), Jump),
                (Pad(PadButton::RightStick), Sneak),
                (Pad(PadButton::East), Sneak),
                (Pad(PadButton::LeftStick), Sprint),
                (Pad(PadButton::Start), Pause),
                (Pad(PadButton::R2), Attack),
                (Pad(PadButton::L2), Use),
                (Pad(PadButton::West), PickBlock),
                (Pad(PadButton::North), Inventory),
                (Pad(PadButton::R1), HotbarNext),
                (Key(KeyCode::ArrowUp), MenuUp),
                (Key(KeyCode::ArrowDown), MenuDown),
                (Key(KeyCode::ArrowLeft), MenuLeft),
                (Key(KeyCode::ArrowRight), MenuRight),
                (Key(KeyCode::Enter), MenuConfirm),
                (Key(KeyCode::Escape), MenuBack),
                (Key(KeyCode::Backspace), Erase),
                (Pad(PadButton::DPadUp), MenuUp),
                (Pad(PadButton::DPadDown), MenuDown),
                (Pad(PadButton::DPadLeft), MenuLeft),
                (Pad(PadButton::DPadRight), MenuRight),
                (Pad(PadButton::South), MenuConfirm),
                (Pad(PadButton::East), MenuBack),
                (Pad(PadButton::L1), HotbarPrevious),
            ],
        }
    }
}
