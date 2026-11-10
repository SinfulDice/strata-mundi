//! The sounds still playing somewhere, so their muffling follows the listener: the path
//! to each is checked again a few times per second.

use super::mixer::{Command, MAX_VOICES};
use super::surroundings::{Surroundings, muffling};
use glam::DVec3;

/// Seconds between two checks of every path.
const CHECK_EVERY: f32 = 0.1;
/// Smaller changes are not sent.
const NOTICEABLE: f32 = 0.02;

struct Tracked {
    id: u32,
    at: DVec3,
    /// Seconds before it ends.
    left: f32,
    muffle: f32,
}

#[derive(Default)]
pub struct MuffleTracker {
    voices: Vec<Tracked>,
    due: f32,
}

impl MuffleTracker {
    pub fn add(&mut self, id: u32, at: DVec3, seconds: f32, muffle: f32) {
        if self.voices.len() == MAX_VOICES {
            self.voices.remove(0);
        }
        self.voices.push(Tracked {
            id,
            at,
            left: seconds,
            muffle,
        });
    }

    pub fn clear(&mut self) {
        self.voices.clear();
    }

    /// Forgets the sounds that ended; every `CHECK_EVERY`, sends the new muffling of
    /// those whose path changed.
    pub fn update(
        &mut self,
        seconds: f32,
        world: &impl Surroundings,
        ears: DVec3,
        send: &mut impl FnMut(Command),
    ) {
        for voice in &mut self.voices {
            voice.left -= seconds;
        }
        self.voices.retain(|voice| voice.left > 0.0);
        self.due -= seconds;
        if self.due > 0.0 {
            return;
        }
        self.due = CHECK_EVERY;
        for voice in &mut self.voices {
            let muffle = muffling(world, voice.at, ears);
            if (muffle - voice.muffle).abs() >= NOTICEABLE {
                voice.muffle = muffle;
                send(Command::Muffle {
                    id: voice.id,
                    muffle,
                });
            }
        }
    }
}
