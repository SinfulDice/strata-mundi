//! Settings of the game itself rather than of its display.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    /// Look for a newer version at launch (one request to GitHub, nothing sent).
    pub check_updates: bool,
    /// Minutes between automatic saves; 0 = only when leaving.
    pub autosave_minutes: u32,
    /// Backup copies kept per world; 0 = none.
    pub backups_kept: u32,
    /// Render only a few frames per second while the window is in the background.
    pub idle_when_unfocused: bool,
    /// Enabled resource packs (folder names), lowest priority first.
    pub resource_packs: Vec<String>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            check_updates: true,
            autosave_minutes: 5,
            backups_kept: 3,
            idle_when_unfocused: true,
            resource_packs: Vec::new(),
        }
    }
}
