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
    Pause,
}

impl Action {
    pub const COUNT: usize = 8;

    pub const fn index(self) -> usize {
        self as usize
    }
}
