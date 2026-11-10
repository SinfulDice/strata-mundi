//! What the audio thread lets go is freed elsewhere: freeing memory may take a lock of
//! the allocator and must not happen while the device waits for samples. Without a
//! trash (tests, a full one), things are freed in place.

use super::stream::Stream;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;

/// Things let go between two emptyings by the game (a few per block at most).
const CAPACITY: usize = 1024;

/// Only held, to be dropped by the game.
pub(super) enum Garbage {
    Buffer { _held: Arc<[f32]> },
    Chunk { _held: Vec<f32> },
    Music { _held: Stream },
}

#[derive(Default)]
pub(super) struct Trash(Option<Sender<Garbage>>);

impl Trash {
    /// The trash and the bin the game empties.
    pub(super) fn new() -> (Self, Receiver<Garbage>) {
        let (sender, receiver) = crossbeam_channel::bounded(CAPACITY);
        (Self(Some(sender)), receiver)
    }

    pub(super) fn throw(&self, garbage: Garbage) {
        if let Some(sender) = &self.0 {
            let _ = sender.try_send(garbage);
        }
    }

    pub(super) fn buffer(&self, buffer: Arc<[f32]>) {
        self.throw(Garbage::Buffer { _held: buffer });
    }
}
