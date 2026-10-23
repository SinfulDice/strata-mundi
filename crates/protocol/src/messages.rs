use serde::{Deserialize, Serialize};

/// Intents sent by the client. The server validates and decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    Disconnect,
}

/// State broadcast by the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Sent once per tick so the client can interpolate between ticks.
    Tick { tick: u64 },
}
