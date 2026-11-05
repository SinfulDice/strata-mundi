//! The world time as the client shows it: carried on between the server's messages so
//! the sun moves smoothly, and never going backwards except after a jump of `/time`.

use strata_protocol::DAY_TIME_PERIOD;

const TICKS_PER_SECOND: f64 = 20.0;
/// A gap bigger than this (in ticks) is a `/time` jump: the clock follows at once.
const SNAP_TICKS: f64 = 200.0;
/// The clock may run ahead of the last message by this much at most, so it stops
/// shortly after the server stops (pause) instead of drifting away.
const MAX_LEAD_TICKS: f64 = DAY_TIME_PERIOD as f64 * 1.5;
/// Ticks of drift caught up per second of difference, at most doubling the speed.
const CATCH_UP: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayClock {
    shown: f64,
    /// The server's time, estimated from its last message.
    server: f64,
    received: f64,
    running: bool,
}

impl DayClock {
    pub fn at(time: u64, running: bool) -> Self {
        let time = time as f64;
        Self {
            shown: time,
            server: time,
            received: time,
            running,
        }
    }

    /// Ticks since the first sunrise, with the fraction since the last whole tick.
    pub fn time(&self) -> f64 {
        self.shown
    }

    pub fn receive(&mut self, time: u64, running: bool) {
        let time = time as f64;
        self.server = time;
        self.received = time;
        self.running = running;
        if (time - self.shown).abs() > SNAP_TICKS {
            self.shown = time;
        }
    }

    pub fn advance(&mut self, seconds: f32) {
        let ticks = f64::from(seconds.max(0.0)) * TICKS_PER_SECOND;
        let behind = self.server - self.shown;
        if !self.running {
            self.shown += ticks.min(behind.max(0.0));
            return;
        }
        let limit = self.received + MAX_LEAD_TICKS;
        self.server = (self.server + ticks).min(limit);
        let speed = (1.0 + behind * CATCH_UP / TICKS_PER_SECOND).clamp(0.0, 2.0);
        self.shown = self.shown.max((self.shown + ticks * speed).min(limit));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_goes_on_smoothly_and_never_back_between_messages() {
        let mut clock = DayClock::at(1_000, true);
        let mut previous = clock.time();
        let mut server = 1_000;
        for frame in 1..=600 {
            // 60 frames a second, a message every second a little late or early.
            clock.advance(1.0 / 60.0);
            if frame % 60 == 0 {
                server += DAY_TIME_PERIOD + if frame % 120 == 0 { 2 } else { 0 };
                clock.receive(server, true);
            }
            let now = clock.time();
            assert!(now >= previous, "back in time at frame {frame}");
            assert!(now - previous < 1.0, "jump at frame {frame}");
            previous = now;
        }
        assert!((clock.time() - server as f64).abs() < 20.0);
    }

    #[test]
    fn the_clock_stops_soon_after_the_server_and_follows_jumps() {
        let mut clock = DayClock::at(0, true);
        for _ in 0..600 {
            clock.advance(0.1);
        }
        assert!(clock.time() <= MAX_LEAD_TICKS + 1e-9, "paused server");
        clock.receive(18_000, false);
        assert_eq!(clock.time(), 18_000.0);
        clock.advance(1.0);
        assert_eq!(clock.time(), 18_000.0, "stopped cycle");
    }
}
