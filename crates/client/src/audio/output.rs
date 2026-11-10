//! The audio device (through `rodio`): one endless stereo source that asks the mixer for
//! blocks of samples and takes the game's commands between blocks.

use super::controls::{Applied, Controls};
use super::mixer::{Command, Mixer, Volumes};
use super::trash::{Garbage, Trash};
use crossbeam_channel::{Receiver, Sender};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Source};
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

/// Frames rendered at once: about 12 ms at 44.1 kHz (the delay of a command).
const BLOCK_FRAMES: usize = 512;
/// Commands waiting at most (sounds asked for while the device stalls are dropped).
const QUEUE: usize = 1024;
const STEREO: usize = 2;

/// An open device; dropping it stops the sound.
pub struct Output {
    _sink: MixerDeviceSink,
    commands: Sender<Command>,
    controls: Arc<Controls>,
    trash: Receiver<Garbage>,
}

impl Output {
    /// The default device, or `None` (and a line in the log) if there is none.
    pub fn open(volumes: Volumes, spatial: bool) -> Option<Self> {
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
        let controls = Arc::new(Controls::new(volumes, spatial));
        let (bin, trash) = Trash::new();
        let mixer = Mixer::with_trash(rate.get(), bin);
        let source = MixerSource::new(mixer, receiver, Arc::clone(&controls), rate);
        sink.mixer().add(source);
        tracing::info!(rate = rate.get(), "audio device open");
        Some(Self {
            _sink: sink,
            commands,
            controls,
            trash,
        })
    }

    /// Frees what the audio thread let go.
    pub fn empty_trash(&self) {
        for garbage in self.trash.try_iter() {
            drop(garbage);
        }
    }

    /// False if the command was dropped: the queue is full because the device is stuck
    /// (losing a sound is better than waiting). Controls (ears, volumes, space) are never
    /// dropped: the last value is kept.
    pub fn send(&self, command: Command) -> bool {
        match self.controls.take(command) {
            Some(command) => self.commands.try_send(command).is_ok(),
            None => true,
        }
    }
}

struct MixerSource {
    mixer: Mixer,
    commands: Receiver<Command>,
    controls: Arc<Controls>,
    applied: Option<Applied>,
    block: Vec<f32>,
    cursor: usize,
    rate: NonZero<u32>,
}

impl MixerSource {
    fn new(
        mixer: Mixer,
        commands: Receiver<Command>,
        controls: Arc<Controls>,
        rate: NonZero<u32>,
    ) -> Self {
        let block = vec![0.0; BLOCK_FRAMES * STEREO];
        Self {
            mixer,
            commands,
            controls,
            applied: None,
            cursor: block.len(),
            block,
            rate,
        }
    }

    fn follow_controls(&mut self) {
        let now = self.controls.read();
        let mixer = &mut self.mixer;
        now.changes(self.applied.as_ref(), &mut |command| mixer.apply(command));
        self.applied = Some(now);
    }
}

impl Iterator for MixerSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.cursor >= self.block.len() {
            self.follow_controls();
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
        NonZero::<u16>::MIN.saturating_add(STEREO as u16 - 1)
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
