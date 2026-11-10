//! The audio device (through `rodio`): one endless stereo source that asks the mixer for
//! blocks of samples and takes the game's commands between blocks.

use super::mixer::{Command, Mixer};
use crossbeam_channel::{Receiver, Sender};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Source};
use std::num::NonZero;
use std::time::Duration;

/// Frames rendered at once: about 12 ms at 44.1 kHz (the delay of a command).
const BLOCK_FRAMES: usize = 512;
/// Commands waiting at most (sounds asked for while the device stalls are dropped).
const QUEUE: usize = 1024;

/// An open device; dropping it stops the sound.
pub struct Output {
    _sink: MixerDeviceSink,
    commands: Sender<Command>,
}

impl Output {
    /// The default device, or `None` (and a line in the log) if there is none.
    pub fn open() -> Option<Self> {
        let mut sink = match DeviceSinkBuilder::open_default_sink() {
            Ok(sink) => sink,
            Err(error) => {
                tracing::info!(%error, "no audio device: playing without sound");
                return None;
            }
        };
        sink.log_on_drop(false);
        let rate = sink.config().sample_rate();
        let (commands, receiver) = crossbeam_channel::bounded(QUEUE);
        sink.mixer()
            .add(MixerSource::new(Mixer::new(rate.get()), receiver, rate));
        tracing::info!(rate = rate.get(), "audio device open");
        Some(Self {
            _sink: sink,
            commands,
        })
    }

    pub fn send(&self, command: Command) {
        // Full queue: the device is stuck, losing a sound is better than waiting.
        let _ = self.commands.try_send(command);
    }
}

struct MixerSource {
    mixer: Mixer,
    commands: Receiver<Command>,
    block: Vec<f32>,
    cursor: usize,
    rate: NonZero<u32>,
}

impl MixerSource {
    fn new(mixer: Mixer, commands: Receiver<Command>, rate: NonZero<u32>) -> Self {
        let block = vec![0.0; BLOCK_FRAMES * 2];
        Self {
            mixer,
            commands,
            cursor: block.len(),
            block,
            rate,
        }
    }
}

impl Iterator for MixerSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.cursor >= self.block.len() {
            for command in self.commands.try_iter() {
                self.mixer.apply(command);
            }
            self.mixer.render(&mut self.block);
            self.cursor = 0;
        }
        let sample = self.block[self.cursor];
        self.cursor += 1;
        Some(sample)
    }
}

impl Source for MixerSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::<u16>::MIN.saturating_add(1)
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
