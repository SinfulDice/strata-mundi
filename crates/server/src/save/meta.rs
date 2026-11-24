use super::stacks::SavedInventory;
use serde::{Deserialize, Serialize};
use strata_core::generation::WorldgenSettings;
use strata_core::id::Identifier;
use strata_core::inventory::DurabilityMode;
use strata_core::physics::Body;
use strata_core::survival::SurvivalOptions;
use strata_core::updates::{FireSettings, FluidSettings};
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
    /// World time in ticks since the first sunrise: the days go on counting (moon phases).
    #[serde(default)]
    pub day_time: u64,
    /// World option: the time of day goes on (otherwise it stays where `/time` put it).
    #[serde(default = "yes")]
    pub day_cycle: bool,
    /// World option: liquids flow (and how many update per tick).
    #[serde(default)]
    pub fluids: FluidSettings,
    /// World option: fire spreads and destroys what burns.
    #[serde(default)]
    pub fire: FireSettings,
    /// World option: cheating commands (`/give`, `/gamemode`…) may be used. Absent in
    /// older worlds: allowed in creative ones (see `commands_allowed`).
    #[serde(default)]
    pub commands: Option<bool>,
    /// World option: recipes are learnt by playing; otherwise all are known.
    #[serde(default = "yes")]
    pub discover_recipes: bool,
    /// World option: worn-out items break, or stay useless until repaired.
    #[serde(default)]
    pub durability: DurabilityMode,
    /// World option: equipment may give movement abilities (double jump, dash, climbing).
    #[serde(default = "yes")]
    pub gear_mobility: bool,
    /// World option: magnet upgrades of chests and barrels draw the items lying around.
    #[serde(default = "yes")]
    pub storage_magnets: bool,
    /// World options: hunger, thirst and stamina, and how fast they go down.
    #[serde(default)]
    pub survival: SurvivalOptions,
    /// World type and generation settings; older worlds are of the pack's normal type.
    #[serde(default)]
    pub generation: WorldgenSettings,
}

fn yes() -> bool {
    true
}

impl WorldMeta {
    /// Whether cheating commands may be used; an older world decides once, from its mode.
    pub fn commands_allowed(&mut self) -> bool {
        *self.commands.get_or_insert(self.mode == GameMode::Creative)
    }
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
    /// The inventory (v0.3); older worlds only have `hotbar`.
    #[serde(default)]
    pub inventory: Option<SavedInventory>,
    /// Hotbar of worlds saved before v0.3: item identifiers, one of each.
    #[serde(default)]
    pub hotbar: Vec<Option<Identifier>>,
    #[serde(default)]
    pub selected: usize,
    /// The boat the player sits in: its index in the dimension's saved boats.
    #[serde(default)]
    pub boat: Option<usize>,
    /// Recipes learnt, by identifier (the starting ones are added at every load).
    #[serde(default)]
    pub recipes: Vec<Identifier>,
    /// Health, needs and effects (v0.3); older worlds start with full gauges.
    #[serde(default)]
    pub vitals: Option<SavedVitals>,
}

/// The survival gauges of a player as saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedVitals {
    pub health: f32,
    pub needs: strata_core::survival::Needs,
    #[serde(default)]
    pub effects: strata_core::survival::StatusEffects,
}
