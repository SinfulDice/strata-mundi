//! The kind of place around the ears, guessed from a few rays cast over several frames:
//! it picks the reverberation.

use super::surroundings::{Surroundings, WALL};
use super::triggers::cell;
use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Space {
    #[default]
    Outdoors,
    /// Walls and a roof close by, with the surface just above (a house).
    Interior,
    SmallCave,
    LargeCavern,
}

impl Space {
    pub const ALL: [Self; 4] = [
        Self::Outdoors,
        Self::Interior,
        Self::SmallCave,
        Self::LargeCavern,
    ];

    /// Its file in the packs' `spaces/`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Outdoors => "outdoors",
            Self::Interior => "interior",
            Self::SmallCave => "small_cave",
            Self::LargeCavern => "large_cavern",
        }
    }
}

/// Rays towards the 26 neighbours of a cell (faces, edges and corners).
const RAYS: usize = 26;
/// Rays cast per second: the whole set about ten times a second, whatever the frame rate.
const RAYS_PER_SECOND: f32 = 260.0;
/// Blocks a ray goes at most; a ray that gets there escaped.
pub const REACH: f32 = 48.0;
/// Rays at or above the horizon that must escape for the open air.
const OPEN_SHARE: f32 = 0.4;
/// Mean reach (blocks) above which a closed space is large.
const LARGE: f32 = 9.0;

fn direction(index: usize) -> DVec3 {
    let index = if index >= 13 { index + 1 } else { index } as i32;
    let (x, y, z) = (index % 3 - 1, index / 3 % 3 - 1, index / 9 - 1);
    DVec3::new(f64::from(x), f64::from(y), f64::from(z)).normalize()
}

/// Blocks before a wall along `direction`, or `REACH`.
fn reach(world: &impl Surroundings, from: DVec3, direction: DVec3) -> f32 {
    let start = cell(from);
    for step in 1..=REACH as i32 {
        let here = cell(from + direction * f64::from(step));
        if here != start && world.density(here) >= WALL {
            return step as f32;
        }
    }
    REACH
}

#[derive(Debug, Clone)]
pub struct SpaceProbe {
    reaches: [f32; RAYS],
    next: usize,
    /// Rays owed by the time gone by (a fraction is carried to the next update).
    owed: f32,
    space: Space,
}

impl Default for SpaceProbe {
    fn default() -> Self {
        Self {
            reaches: [REACH; RAYS],
            next: 0,
            owed: 0.0,
            space: Space::Outdoors,
        }
    }
}

impl SpaceProbe {
    /// Casts the rays due after `seconds`; once all were cast, the space is judged again.
    pub fn update(
        &mut self,
        seconds: f32,
        world: &impl Surroundings,
        head: DVec3,
        underground: f32,
    ) -> Space {
        self.owed = (self.owed + seconds.max(0.0) * RAYS_PER_SECOND).min(RAYS as f32);
        let rays = self.owed as usize;
        self.owed -= rays as f32;
        for _ in 0..rays {
            self.reaches[self.next] = reach(world, head, direction(self.next));
            self.next += 1;
            if self.next == RAYS {
                self.next = 0;
                let sky = world.sky_light(cell(head));
                self.space = classify(&self.reaches, sky, underground);
            }
        }
        self.space
    }

    pub fn space(&self) -> Space {
        self.space
    }
}

/// Open air where the sky is seen (or most rays upwards escape under some sky light),
/// otherwise a room, a cave or a cavern by the mean distance to the walls.
fn classify(reaches: &[f32; RAYS], sky: u8, underground: f32) -> Space {
    let upwards = (0..RAYS).filter(|&i| direction(i).y >= 0.0);
    let (count, escaped) = upwards.fold((0, 0), |(count, escaped), i| {
        (count + 1, escaped + usize::from(reaches[i] >= REACH))
    });
    let open = escaped as f32 / count.max(1) as f32;
    if sky >= 15 || (open >= OPEN_SHARE && sky >= 8) {
        return Space::Outdoors;
    }
    let mean = reaches.iter().sum::<f32>() / RAYS as f32;
    if mean >= LARGE {
        Space::LargeCavern
    } else if underground >= 0.5 {
        Space::SmallCave
    } else {
        Space::Interior
    }
}
