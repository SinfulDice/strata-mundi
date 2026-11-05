use std::time::{Duration, Instant};

pub const TICKS_PER_SECOND: u32 = 20;
pub const TICK_DURATION: Duration = Duration::from_millis(1000 / TICKS_PER_SECOND as u64);

/// Fixed-step scheduler: says how many ticks are due, never drifts,
/// and drops the backlog after a long stall instead of fast-forwarding.
pub struct TickClock {
    next_tick: Instant,
}

const MAX_CATCH_UP_TICKS: u32 = 10;

impl TickClock {
    pub fn new(now: Instant) -> Self {
        Self { next_tick: now }
    }

    pub fn due_ticks(&mut self, now: Instant) -> u32 {
        let mut due = 0;
        while self.next_tick <= now && due < MAX_CATCH_UP_TICKS {
            self.next_tick += TICK_DURATION;
            due += 1;
        }
        if self.next_tick <= now {
            self.next_tick = now + TICK_DURATION;
        }
        due
    }

    pub fn next_tick(&self) -> Instant {
        self.next_tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_tick_every_50_ms() {
        let start = Instant::now();
        let mut clock = TickClock::new(start);
        assert_eq!(clock.due_ticks(start), 1);
        assert_eq!(clock.due_ticks(start + Duration::from_millis(49)), 0);
        assert_eq!(clock.due_ticks(start + Duration::from_millis(50)), 1);
        assert_eq!(clock.due_ticks(start + Duration::from_millis(200)), 3);
    }

    #[test]
    fn long_stall_is_capped() {
        let start = Instant::now();
        let mut clock = TickClock::new(start);
        assert_eq!(
            clock.due_ticks(start + Duration::from_secs(60)),
            MAX_CATCH_UP_TICKS
        );
        assert!(clock.next_tick() > start + Duration::from_secs(60));
    }
}
