//! Every exchange between client and server is a serializable message, even in single player,
//! so that multiplayer (v2.0) only has to plug a network transport.

mod local;
mod messages;

pub use local::{ClientConnection, Disconnected, ServerConnection, local_pair};
pub use messages::{ClientMessage, ServerMessage};

/// Bounds of `ClientMessage::ViewDistance`, in columns.
pub const MIN_VIEW_DISTANCE: u32 = 2;
pub const MAX_VIEW_DISTANCE: u32 = 32;

/// Bumped whenever a message changes in an incompatible way.
pub const PROTOCOL_VERSION: u32 = 1;
