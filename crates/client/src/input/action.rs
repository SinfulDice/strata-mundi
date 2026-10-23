/// What the player wants to do, independent of the device used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Sneak,
    Sprint,
    Attack,
    Use,
    PickBlock,
    Inventory,
    HotbarNext,
    HotbarPrevious,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
    Hotbar9,
    Pause,
    RenderDistanceDown,
    RenderDistanceUp,
    MenuUp,
    MenuDown,
    MenuLeft,
    MenuRight,
    MenuConfirm,
    MenuBack,
    Erase,
}

impl Action {
    pub const COUNT: usize = 32;
    pub const ALL: [Action; Self::COUNT] = [
        Self::MoveForward,
        Self::MoveBackward,
        Self::MoveLeft,
        Self::MoveRight,
        Self::Jump,
        Self::Sneak,
        Self::Sprint,
        Self::Attack,
        Self::Use,
        Self::PickBlock,
        Self::Inventory,
        Self::HotbarNext,
        Self::HotbarPrevious,
        Self::Hotbar1,
        Self::Hotbar2,
        Self::Hotbar3,
        Self::Hotbar4,
        Self::Hotbar5,
        Self::Hotbar6,
        Self::Hotbar7,
        Self::Hotbar8,
        Self::Hotbar9,
        Self::Pause,
        Self::RenderDistanceDown,
        Self::RenderDistanceUp,
        Self::MenuUp,
        Self::MenuDown,
        Self::MenuLeft,
        Self::MenuRight,
        Self::MenuConfirm,
        Self::MenuBack,
        Self::Erase,
    ];

    /// Names used in `<key:…>` markup, in the order of `ALL`.
    const NAMES: [&'static str; Self::COUNT] = [
        "MoveForward",
        "MoveBackward",
        "MoveLeft",
        "MoveRight",
        "Jump",
        "Sneak",
        "Sprint",
        "Attack",
        "Use",
        "PickBlock",
        "Inventory",
        "HotbarNext",
        "HotbarPrevious",
        "Hotbar1",
        "Hotbar2",
        "Hotbar3",
        "Hotbar4",
        "Hotbar5",
        "Hotbar6",
        "Hotbar7",
        "Hotbar8",
        "Hotbar9",
        "Pause",
        "RenderDistanceDown",
        "RenderDistanceUp",
        "MenuUp",
        "MenuDown",
        "MenuLeft",
        "MenuRight",
        "MenuConfirm",
        "MenuBack",
        "Erase",
    ];

    /// By its name in the code (`Inventory`), as written in `<key:…>` markup.
    pub fn from_name(name: &str) -> Option<Self> {
        let index = Self::NAMES.iter().position(|n| *n == name)?;
        Some(Self::ALL[index])
    }
    pub const HOTBAR: [Action; 9] = [
        Self::Hotbar1,
        Self::Hotbar2,
        Self::Hotbar3,
        Self::Hotbar4,
        Self::Hotbar5,
        Self::Hotbar6,
        Self::Hotbar7,
        Self::Hotbar8,
        Self::Hotbar9,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_is_listed_once_in_order() {
        for (index, action) in Action::ALL.into_iter().enumerate() {
            assert_eq!(action.index(), index);
            assert_eq!(Action::from_name(&format!("{action:?}")), Some(action));
        }
    }
}
