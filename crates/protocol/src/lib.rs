//! Every exchange between client and server is a serializable message, even in single player,
//! so that multiplayer (v2.0) only has to plug a network transport.

pub mod commands;
mod local;
mod messages;
mod worlds;

pub use local::{ClientConnection, Disconnected, ServerConnection, local_pair};
pub use messages::{
    ActionId, BiomeInfo, ClientMessage, DAY_TIME_PERIOD, DayState, EntityState, HotbarState,
    MAX_PENDING_ACTIONS, NetEntity, ServerMessage, WorldAction,
};
pub use worlds::{GameMode, NewWorld, WorldStatus, WorldSummary};

/// Bounds of `ClientMessage::ViewDistance`, in columns: the client asks for a little
/// more than it draws (its own margin).
pub const MIN_VIEW_DISTANCE: u32 = 2;
pub const MAX_VIEW_DISTANCE: u32 = 40;

/// Whether the column `(dx, dz)` away from the player's lies in the loaded disc.
pub fn in_view_disc(dx: i32, dz: i32, view_distance: u32) -> bool {
    let r = view_distance as i32;
    dx * dx + dz * dz <= r * r + r
}

/// Bumped whenever a message changes in an incompatible way.
pub const PROTOCOL_VERSION: u32 = 8;
