//! The open furnace's flame and arrow between two messages: the server only writes when
//! the furnace leaves its course, so the client moves it on by itself, tick by tick.

use strata_protocol::OpenedContainer;

/// Seconds of one server tick.
const TICK: f32 = 1.0 / 20.0;

#[derive(Debug, Default)]
pub struct FurnaceClock {
    elapsed: f32,
}

impl FurnaceClock {
    /// A view just came from the server: it is the state now.
    pub fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    pub fn follow(&mut self, open: Option<&mut OpenedContainer>, dt: f32) {
        let Some(view) = open.and_then(|open| open.furnace.as_mut()) else {
            self.elapsed = 0.0;
            return;
        };
        self.elapsed += dt.clamp(0.0, 1.0);
        while self.elapsed >= TICK - 1e-4 {
            self.elapsed -= TICK;
            *view = view.advanced();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_core::content::BlockId;
    use strata_core::coords::BlockPos;
    use strata_core::inventory::Container;
    use strata_protocol::FurnaceView;

    #[test]
    fn the_flame_burns_down_and_the_arrow_goes_on_between_messages() {
        let mut open = OpenedContainer {
            block: BlockId(1),
            pos: BlockPos::new(0, 0, 0),
            contents: Container::new(3),
            furnace: Some(FurnaceView {
                burn_left: 100,
                burn_total: 100,
                progress: 0,
                cook_time: 200,
                seconds: 10.0,
                per_coal: Some(8.0),
            }),
        };
        let mut clock = FurnaceClock::default();
        clock.follow(Some(&mut open), 1.0);
        let view = open.furnace.unwrap();
        assert_eq!(
            (view.burn_left, view.progress),
            (80, 20),
            "one second: 20 ticks"
        );
    }
}
