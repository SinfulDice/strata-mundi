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
