//! Authoritative simulation. The client only sends intents; the server decides.

mod clock;
mod ecs;
mod game;
mod generation;
mod integrated;
mod loader;
mod persistence;
mod rules;
pub mod save;
mod systems;

pub use clock::{TICK_DURATION, TICKS_PER_SECOND, TickClock};
pub use integrated::{IntegratedServer, ServerConfig};
pub use persistence::WorldSettings;
pub use rules::BlockRules;
