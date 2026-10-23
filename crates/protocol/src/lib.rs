//! Every exchange between client and server is a serializable message, even in single player,
//! so that multiplayer (v2.0) only has to plug a network transport.

pub mod commands;
mod local;
mod messages;
mod worlds;

pub use local::{ClientConnection, Disconnected, ServerConnection, local_pair};
pub use messages::{ClientMessage, HotbarState, ServerMessage};
pub use worlds::{GameMode, NewWorld, WorldStatus, WorldSummary};

/// Bounds of `ClientMessage::ViewDistance`, in columns.
pub const MIN_VIEW_DISTANCE: u32 = 2;
pub const MAX_VIEW_DISTANCE: u32 = 32;

/// Bumped whenever a message changes in an incompatible way.
pub const PROTOCOL_VERSION: u32 = 2;
