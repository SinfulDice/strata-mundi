use serde::{Deserialize, Serialize};
use strata_core::id::Identifier;
use strata_core::physics::Body;
use strata_protocol::GameMode;

/// Bumped on any incompatible change of the files of a world. No migration before v1.0:
/// an older world gets a clear message instead of a crash.
pub const FORMAT_VERSION: u32 = 1;

/// `world.ron`: what the world list shows and what the server needs to start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "World")]
pub struct WorldMeta {
    pub format: u32,
    pub game_version: String,
    pub name: String,
    pub seed: u64,
    /// As typed by the player (shown by `/seed`).
    pub seed_text: String,
    pub mode: GameMode,
    pub border: i32,
    /// Seconds since the Unix epoch.
    pub created: i64,
    pub last_played: i64,
    pub play_ticks: u64,
}

/// Only the version, readable whatever the rest of the file looks like.
#[derive(Debug, Deserialize)]
#[serde(rename = "World")]
pub struct FormatOnly {
    pub format: u32,
}

/// `player.ron`: the one character of the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Player")]
pub struct PlayerData {
    pub dimension: Identifier,
    pub body: Body,
    /// Yaw and pitch, in radians.
    #[serde(default)]
    pub look: [f32; 2],
    /// Item identifiers, so a world survives packs being added or reordered.
    pub hotbar: Vec<Option<Identifier>>,
    pub selected: usize,
}
