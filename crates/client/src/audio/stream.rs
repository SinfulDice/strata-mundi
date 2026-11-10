//! Music in the mixer: a piece arrives in chunks from a reader thread (never the whole
//! piece in memory), is faded in and out, centred, at the music volume. A missing chunk
//! (disk too slow) is a moment of silence, never a wait in the audio thread. Played
//! chunks go back to the reader to be filled again.

use super::mixer::Volumes;
use super::trash::{Garbage, Trash};
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use strata_core::audio::SoundCategory;

/// Seconds of a fade in or out.
pub const MUSIC_FADE: f32 = 3.0;

/// The chunks of one piece, and a flag raised once it is over or let go (also when the
/// feed is dropped anywhere, a lost command included).
#[derive(Debug)]
pub struct MusicFeed {
    pub chunks: Receiver<Vec<f32>>,
    /// Played chunks, given back to the reader.
    pub recycle: Sender<Vec<f32>>,
    pub done: Arc<AtomicBool>,
}

impl Drop for MusicFeed {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Release);
    }
}

#[derive(Debug)]
pub enum MusicCommand {
    /// Plays a piece (faded in), letting go the one before.
    Start(MusicFeed),
    /// Fades the piece out, then lets it go.
    FadeOut,
}

pub(super) struct Stream {
    feed: MusicFeed,
    chunk: Vec<f32>,
    cursor: f64,
    gain: f32,
    fading_out: bool,
}

impl Stream {
    pub fn apply(slot: &mut Option<Stream>, command: MusicCommand, trash: &Trash) {
        match command {
            MusicCommand::Start(feed) => {
                if let Some(old) = slot.take() {
                    old.finish(trash);
                }
                *slot = Some(Stream {
                    feed,
                    chunk: Vec::new(),
                    cursor: 0.0,
                    gain: 0.0,
                    fading_out: false,
                });
            }
            MusicCommand::FadeOut => {
                if let Some(stream) = slot {
                    stream.fading_out = true;
                }
            }
        }
    }

    /// Adds the piece to `out` (interleaved stereo); lets it go once over.
    /// `step`: samples of the piece per output frame; `rate`: output frames per second.
    pub fn mix(
        slot: &mut Option<Stream>,
        volumes: &Volumes,
        out: &mut [f32],
        (step, rate): (f64, f32),
        trash: &Trash,
    ) {
        let Some(stream) = slot else {
            return;
        };
        let volume = volumes.get(SoundCategory::Music) * std::f32::consts::FRAC_1_SQRT_2;
        let fade = 1.0 / (MUSIC_FADE * rate).max(1.0);
        for frame in 0..out.len() / 2 {
            let Some(sample) = stream.next_sample(step, trash) else {
                if let Some(over) = slot.take() {
                    over.finish(trash);
                }
                return;
            };
            let target = if stream.fading_out { 0.0 } else { 1.0 };
            stream.gain += (target - stream.gain).clamp(-fade, fade);
            let value = sample * stream.gain * volume;
            out[frame * 2] += value;
            out[frame * 2 + 1] += value;
            if stream.fading_out && stream.gain <= 0.0 {
                if let Some(over) = slot.take() {
                    over.finish(trash);
                }
                return;
            }
        }
    }

    /// The next sample; `None` once the piece is over (`Some(0)` while a chunk is late).
    fn next_sample(&mut self, step: f64, trash: &Trash) -> Option<f32> {
        while self.cursor as usize >= self.chunk.len() {
            match self.feed.chunks.try_recv() {
                Ok(chunk) => {
                    self.cursor -= self.chunk.len() as f64;
                    let played = std::mem::replace(&mut self.chunk, chunk);
                    self.recycle(played, trash);
                }
                Err(TryRecvError::Empty) => return Some(0.0),
                Err(TryRecvError::Disconnected) => return None,
            }
        }
        let index = self.cursor as usize;
        let fraction = (self.cursor - index as f64) as f32;
        let a = self.chunk[index];
        let b = self.chunk.get(index + 1).copied().unwrap_or(a);
        self.cursor += step;
        Some(a + (b - a) * fraction)
    }

    fn recycle(&self, chunk: Vec<f32>, trash: &Trash) {
        if chunk.capacity() == 0 {
            return;
        }
        if let Err(back) = self.feed.recycle.try_send(chunk) {
            trash.throw(Garbage::Chunk {
                _held: back.into_inner(),
            });
        }
    }

    fn finish(self, trash: &Trash) {
        self.feed.done.store(true, Ordering::Release);
        trash.throw(Garbage::Music { _held: self });
    }
}
