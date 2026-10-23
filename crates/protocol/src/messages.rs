use serde::{Deserialize, Serialize};
use strata_core::chunk::Column;
use strata_core::content::{BlockId, Face};
use strata_core::coords::{BlockPos, ColumnPos};
use strata_core::dimension::DimensionId;
use strata_core::physics::{Body, MoveInput, MovementSettings};

/// Intents sent by the client. The server validates and decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// What the player wants during one tick, numbered so the client can match the
    /// server's answer to its own prediction.
    Input {
        sequence: u32,
        input: MoveInput,
    },
    /// Columns to keep loaded around the player, in each direction
    /// (`MIN_VIEW_DISTANCE..=MAX_VIEW_DISTANCE`).
    ViewDistance(u32),
    /// Comfort options from the client's settings.
    MovementPreferences {
        auto_jump: bool,
        step_up: bool,
    },
    /// Creative only: move the player instantly (benchmarks, `/tp` from milestone 8).
    Teleport {
        feet: [f64; 3],
    },
    BreakBlock {
        pos: BlockPos,
    },
    /// Put `block` against `face` of the block at `against`.
    PlaceBlock {
        against: BlockPos,
        face: Face,
        block: BlockId,
    },
    Disconnect,
}

/// State broadcast by the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// First message: where and how the player starts.
    Welcome {
        dimension: DimensionId,
        body: Body,
        movement: MovementSettings,
        /// Half-width of the world, in blocks.
        border: i32,
    },
    /// Sent once per tick so the client can interpolate between ticks.
    Tick {
        tick: u64,
    },
    /// Authoritative state of the player after applying input `sequence`.
    PlayerState {
        sequence: u32,
        body: Body,
    },
    /// A column of blocks entering the player's view distance.
    Column(Box<Column>),
    /// A column leaving the view distance: the client forgets it.
    UnloadColumn(ColumnPos),
    BlockChanged {
        pos: BlockPos,
        block: BlockId,
    },
}
