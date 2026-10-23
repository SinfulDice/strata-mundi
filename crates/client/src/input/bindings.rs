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
            ],
        }
    }
}
