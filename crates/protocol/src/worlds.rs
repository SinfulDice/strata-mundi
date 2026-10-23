//! Worlds as the menus see them: listing, creating, opening.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GameMode {
    /// Every block available, flight, instant breaking. Survival arrives in v0.3.
    #[default]
    Creative,
}

/// What the player chose on the "new world" screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewWorld {
    pub name: String,
    /// Any text; empty = random.
    pub seed: String,
    pub mode: GameMode,
    /// Half-width of the playable square, in blocks.
    pub border: i32,
}

/// Whether a saved world can be opened by this version of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldStatus {
    Ready,
    /// Saved by an older, incompatible version (no migration before v1.0).
    TooOld,
    /// Saved by a newer version of the game.
    TooNew,
    /// The world description is missing or unreadable.
    Broken,
}

/// One line of the world list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSummary {
    /// Folder name, unique.
    pub id: String,
    pub name: String,
    pub mode: GameMode,
    /// Seconds since the Unix epoch.
    pub last_played: i64,
    pub play_seconds: u64,
    /// PNG captured at the last save.
    pub icon: Option<Vec<u8>>,
    pub status: WorldStatus,
}
