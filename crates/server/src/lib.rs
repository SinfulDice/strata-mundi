//! Authoritative simulation. The client only sends intents; the server decides.

mod clock;
mod generation;
mod integrated;
mod loader;

pub use clock::{TICK_DURATION, TICKS_PER_SECOND, TickClock};
pub use integrated::{IntegratedServer, ServerConfig};
