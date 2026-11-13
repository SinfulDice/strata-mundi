//! Holding ← → (keys, d-pad or stick) on a value: one step at once, then after a delay
//! repeated steps that speed up, until released.

/// Wait before the first repeat, in seconds.
pub const REPEAT_DELAY: f32 = 0.4;
/// Interval between the first repeats.
pub const REPEAT_START: f32 = 0.15;
/// Each repeat shortens the interval by this factor…
pub const REPEAT_SPEEDUP: f32 = 0.85;
/// …down to this one.
pub const REPEAT_FASTEST: f32 = 0.03;
/// A stalled frame counts for this long at most, so a hitch does not jump many steps.
const LONGEST_FRAME: f32 = 0.1;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct HoldRepeat {
    /// Direction held last frame (-1, 0 or 1).
    held: i32,
    /// Time left before the next repeat.
    wait: f32,
    interval: f32,
}

impl HoldRepeat {
    /// Steps to apply this frame for `held` (-1 left, 1 right, 0 nothing) after `dt`
    /// seconds: the sign gives the direction.
    pub fn update(&mut self, held: i32, dt: f32) -> i32 {
        let held = held.signum();
        if held == 0 {
            *self = Self::default();
            return 0;
        }
        if held != self.held {
            *self = Self {
                held,
                wait: REPEAT_DELAY,
                interval: REPEAT_START,
            };
            return held;
        }
        self.wait -= dt.clamp(0.0, LONGEST_FRAME);
        let mut steps = 0;
        while self.wait <= 0.0 {
            steps += held;
            self.wait += self.interval;
            self.interval = (self.interval * REPEAT_SPEEDUP).max(REPEAT_FASTEST);
        }
        steps
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    /// Steps given by each frame of holding `held` for `seconds`.
    fn hold(repeat: &mut HoldRepeat, held: i32, seconds: f32) -> Vec<i32> {
        let frames = (seconds / FRAME).round() as usize;
        (0..frames).map(|_| repeat.update(held, FRAME)).collect()
    }

    #[test]
    fn one_step_at_once_then_nothing_until_the_delay() {
        let mut repeat = HoldRepeat::default();
        let steps = hold(&mut repeat, 1, REPEAT_DELAY - 2.0 * FRAME);
        assert_eq!(steps[0], 1);
        assert!(steps[1..].iter().all(|&s| s == 0));
        let after: i32 = hold(&mut repeat, 1, 4.0 * FRAME).iter().sum();
        assert_eq!(after, 1);
    }

    #[test]
    fn repeats_speed_up_to_the_fastest_rate() {
        let mut repeat = HoldRepeat::default();
        let first: i32 = hold(&mut repeat, -1, 1.0).iter().sum();
        let second: i32 = hold(&mut repeat, -1, 1.0).iter().sum();
        let much_later: i32 = {
            hold(&mut repeat, -1, 5.0);
            hold(&mut repeat, -1, 1.0).iter().sum()
        };
        assert!(first < 0 && second < first, "{first} then {second}");
        let fastest = -(1.0 / REPEAT_FASTEST).round() as i32;
        assert!((much_later - fastest).abs() <= 1, "{much_later}");
    }

    #[test]
    fn release_stops_and_a_new_press_starts_over() {
        let mut repeat = HoldRepeat::default();
        hold(&mut repeat, 1, 2.0);
        assert_eq!(repeat.update(0, FRAME), 0);
        assert_eq!(repeat.update(0, 10.0), 0);
        assert_eq!(repeat.update(1, FRAME), 1);
        assert_eq!(repeat.update(1, FRAME), 0);
        assert_eq!(repeat.update(-1, FRAME), -1, "reversing restarts at once");
    }

    #[test]
    fn a_stalled_frame_gives_few_steps() {
        let mut repeat = HoldRepeat::default();
        hold(&mut repeat, 1, 2.0);
        assert!(repeat.update(1, 10.0) <= (LONGEST_FRAME / REPEAT_FASTEST).ceil() as i32);
    }
}
