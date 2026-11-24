use super::Action;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

/// Gamepad buttons, laid out like a New 3DS (ABXY named by position).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    Pad(PadButton),
}

/// Which inputs trigger which action; remappable in the options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "ByAction", into = "ByAction")]
pub struct Bindings {
    entries: Vec<(Binding, Action)>,
}

/// File form: each action with its inputs, easier to read and edit by hand.
type ByAction = BTreeMap<Action, Vec<Binding>>;

/// Order does not matter, only which input triggers which action.
impl PartialEq for Bindings {
    fn eq(&self, other: &Self) -> bool {
        ByAction::from(self.clone()) == ByAction::from(other.clone())
    }
}

/// Actions missing from the file (added by a newer version, or a deleted line) keep
/// their default inputs; an action written with `[]` stays unbound.
impl From<ByAction> for Bindings {
    fn from(map: ByAction) -> Self {
        let before_backpack = !map.is_empty() && !map.contains_key(&Action::Backpack);
        let mut all = ByAction::from(Bindings::default());
        all.extend(map);
        if before_backpack {
            give_dpad_up_to_the_backpack(&mut all);
        }
        let entries = all
            .into_iter()
            .flat_map(|(action, inputs)| inputs.into_iter().map(move |b| (b, action)))
            .collect();
        Self { entries }
    }
}

/// Files from before the backpack key had the d-pad up moving forward: it now opens the
/// backpack, unless the player gave it to another action.
fn give_dpad_up_to_the_backpack(all: &mut ByAction) {
    let up = Binding::Pad(PadButton::DPadUp);
    if let Some(forward) = all.get_mut(&Action::MoveForward) {
        forward.retain(|binding| *binding != up);
    }
    let in_game = |action: &Action| !format!("{action:?}").starts_with("Menu");
    let taken = (all.iter()).any(|(action, inputs)| {
        *action != Action::Backpack && in_game(action) && inputs.contains(&up)
    });
    if taken && let Some(backpack) = all.get_mut(&Action::Backpack) {
        backpack.retain(|binding| *binding != up);
    }
}

impl From<Bindings> for ByAction {
    fn from(bindings: Bindings) -> Self {
        let mut map: ByAction = Action::ALL.iter().map(|&a| (a, Vec::new())).collect();
        for (binding, action) in bindings.entries {
            map.entry(action).or_default().push(binding);
        }
        map
    }
}

impl Bindings {
    pub fn actions_for(&self, binding: Binding) -> impl Iterator<Item = Action> + '_ {
        self.entries
            .iter()
            .filter(move |(b, _)| *b == binding)
            .map(|(_, action)| *action)
    }

    /// Binds `binding` to `action`, replacing the action's previous binding of the same
    /// kind (keyboard and mouse, or gamepad).
    pub fn rebind(&mut self, action: Action, binding: Binding) {
        let pad = matches!(binding, Binding::Pad(_));
        self.entries
            .retain(|(b, a)| *a != action || matches!(b, Binding::Pad(_)) != pad);
        self.entries.push((binding, action));
    }

    /// Takes `binding` away from every action in `actions` but `keep`, so one key does not
    /// end up moving forward and backward at once.
    pub fn release(&mut self, binding: Binding, actions: &[Action], keep: Action) {
        self.entries
            .retain(|(b, a)| *b != binding || *a == keep || !actions.contains(a));
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
                (Key(KeyCode::KeyJ), Journal),
                (Key(KeyCode::KeyR), HammerArea),
                (Key(KeyCode::KeyF), Dash),
                (Key(KeyCode::KeyB), Backpack),
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
                (Key(KeyCode::F3), ToggleDebug),
                (Key(KeyCode::F5), ReloadResources),
                (Key(KeyCode::KeyT), OpenConsole),
                (Key(KeyCode::Slash), OpenConsole),
                (Pad(PadButton::Select), Journal),
                (Pad(PadButton::DPadUp), Backpack),
                (Pad(PadButton::DPadDown), Dash),
                (Pad(PadButton::DPadLeft), MoveLeft),
                (Pad(PadButton::DPadRight), MoveRight),
                (Pad(PadButton::South), Jump),
                (Pad(PadButton::RightStick), HammerArea),
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
                (Key(KeyCode::Tab), Complete),
                (Pad(PadButton::DPadUp), MenuUp),
                (Pad(PadButton::DPadDown), MenuDown),
                (Pad(PadButton::DPadLeft), MenuLeft),
                (Pad(PadButton::DPadRight), MenuRight),
                (Pad(PadButton::South), MenuConfirm),
                (Pad(PadButton::East), MenuBack),
                (Pad(PadButton::L1), HotbarPrevious),
                (Key(KeyCode::KeyQ), Drop),
                (Pad(PadButton::West), MenuSecondary),
                (Pad(PadButton::North), MenuTransfer),
                (Pad(PadButton::RightStick), MenuGather),
                (Pad(PadButton::LeftStick), MenuDrop),
                (Pad(PadButton::L1), MenuLock),
                (Binding::Mouse(MouseButton::Middle), MenuLock),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_file_gives_the_dpad_up_to_the_backpack_if_free() {
        let up = Binding::Pad(PadButton::DPadUp);
        let mut old = ByAction::new();
        old.insert(Action::MoveForward, vec![Binding::Key(KeyCode::KeyW), up]);
        let bindings = Bindings::from(old.clone());
        assert_eq!(
            bindings
                .actions_for(up)
                .filter(|a| !format!("{a:?}").starts_with("Menu"))
                .collect::<Vec<_>>(),
            [Action::Backpack]
        );
        old.insert(Action::Jump, vec![up]);
        let bindings = Bindings::from(old);
        assert!(
            !bindings.bindings_for(Action::Backpack).any(|b| b == up),
            "taken by jump"
        );
    }

    #[test]
    fn rebinding_replaces_only_the_same_kind_of_input() {
        let mut bindings = Bindings::default();
        bindings.rebind(Action::Jump, Binding::Key(KeyCode::KeyJ));
        let jump: Vec<_> = bindings.bindings_for(Action::Jump).collect();
        assert!(jump.contains(&Binding::Key(KeyCode::KeyJ)));
        assert!(!jump.contains(&Binding::Key(KeyCode::Space)));
        assert!(
            jump.contains(&Binding::Pad(PadButton::South)),
            "gamepad kept"
        );
    }

    #[test]
    fn a_key_given_to_one_action_leaves_the_others() {
        let mut bindings = Bindings::default();
        let w = Binding::Key(KeyCode::KeyW);
        let moves = [Action::MoveForward, Action::MoveBackward];
        bindings.release(w, &moves, Action::MoveBackward);
        bindings.rebind(Action::MoveBackward, w);
        assert_eq!(
            bindings.actions_for(w).collect::<Vec<_>>(),
            [Action::MoveBackward]
        );
    }

    #[test]
    fn missing_actions_keep_their_defaults() {
        let file: ByAction = [(Action::Jump, vec![Binding::Key(KeyCode::KeyJ)])].into();
        let bindings = Bindings::from(file);
        assert_eq!(bindings.bindings_for(Action::Jump).count(), 1);
        assert!(bindings.bindings_for(Action::Pause).count() > 0);
    }
}
