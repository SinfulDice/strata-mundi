use serde::{Deserialize, Serialize};
use strata_core::chunk::Column;
use strata_core::coords::ColumnPos;
use strata_core::dimension::DimensionId;

/// Intents sent by the client. The server validates and decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Where the player is and looks (free flight until the player entity, milestone 6).
    PlayerView {
        position: [f64; 3],
        /// Radians, 0 = north (-Z), increasing toward east.
        yaw: f32,
        /// Radians, positive = up.
        pitch: f32,
    },
    /// Columns to keep loaded around the player, in each direction
    /// (`MIN_VIEW_DISTANCE..=MAX_VIEW_DISTANCE`).
    ViewDistance(u32),
    Disconnect,
}

/// State broadcast by the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// First message: where the player starts.
    Welcome {
        dimension: DimensionId,
        spawn: [f64; 3],
    },
    /// Sent once per tick so the client can interpolate between ticks.
    Tick { tick: u64 },
    /// A column of blocks entering the player's view distance.
    Column(Box<Column>),
    /// A column leaving the view distance: the client forgets it.
    UnloadColumn(ColumnPos),
}
