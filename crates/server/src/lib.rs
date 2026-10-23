//! Authoritative simulation. The client only sends intents; the server decides.

use std::time::Duration;

pub const TICKS_PER_SECOND: u32 = 20;
pub const TICK_DURATION: Duration = Duration::from_millis(1000 / TICKS_PER_SECOND as u64);
