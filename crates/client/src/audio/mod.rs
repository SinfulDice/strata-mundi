//! Sound: synthesised (or recorded) buffers played in 3D around the listener, volumes by
//! category, subtitles. Without an audio device everything runs the same, silently.

mod bank;
mod mixer;
mod output;
mod spatial;
mod subtitles;
#[cfg(test)]
mod tests;
mod triggers;

pub use mixer::{Command, MAX_VOICES, Mixer, Voice, Volumes};
pub use spatial::{Direction, Listener, attenuation, stereo_gains};
pub use subtitles::{LIFETIME, MAX_LINES, Subtitle, Subtitles};
pub use triggers::{AmbientBlocks, BlockSounds, Cue, Footsteps, SoundTable};

use crate::settings::SoundSettings;
use crate::world::ClientWorld;
use bank::SoundBank;
use glam::DVec3;
use output::Output;
use strata_core::audio::Rng;
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::physics::Body;

pub struct Audio {
    output: Option<Output>,
    bank: SoundBank,
    table: SoundTable,
    settings: SoundSettings,
    listener: Listener,
    subtitles: Subtitles,
    footsteps: Footsteps,
    ambient: AmbientBlocks,
    rng: Rng,
    cues: Vec<Cue>,
}

impl Audio {
    /// Opens the default device; without one the game goes on in silence.
    pub fn start(content: &Content, settings: &SoundSettings) -> Self {
        Self::with_output(Output::open(), content, settings)
    }

    /// No device at all (captures, tests).
    pub fn silent(content: &Content, settings: &SoundSettings) -> Self {
        Self::with_output(None, content, settings)
    }

    fn with_output(output: Option<Output>, content: &Content, settings: &SoundSettings) -> Self {
        let audio = Self {
            output,
            bank: SoundBank::new(content.sounds.clone()),
            table: SoundTable::from_content(content),
            settings: settings.clone(),
            listener: Listener::default(),
            subtitles: Subtitles::default(),
            footsteps: Footsteps::default(),
            ambient: AmbientBlocks::default(),
            rng: Rng::new(0x50_17D5),
            cues: Vec::new(),
        };
        audio.send(Command::Volumes(Volumes::of(settings)));
        audio
    }

    pub fn has_device(&self) -> bool {
        self.output.is_some()
    }

    /// New content (resources reloaded): buffers are made again from the new recipes.
    pub fn set_content(&mut self, content: &Content) {
        self.bank = SoundBank::new(content.sounds.clone());
        self.table = SoundTable::from_content(content);
    }

    pub fn apply_settings(&mut self, settings: &SoundSettings) {
        if !settings.subtitles {
            self.subtitles.clear();
        }
        self.settings = settings.clone();
        self.send(Command::Volumes(Volumes::of(settings)));
    }

    /// A block changed (server decision): it may have been broken or placed.
    pub fn block_changed(&mut self, pos: BlockPos, old: BlockId, new: BlockId) {
        if let Some(cue) = self.table.changed(pos, old, new) {
            self.cues.push(cue);
        }
    }

    /// Moves the ears, plays the local player's steps and the blocks around, and starts
    /// what was asked this frame. `player` is `None` on the title screen or in a boat.
    pub fn update(
        &mut self,
        seconds: f32,
        listener: Listener,
        player: Option<&Body>,
        world: &ClientWorld,
    ) {
        self.listener = listener;
        self.send(Command::Listener(listener));
        if let Some(body) = player {
            self.footsteps
                .update(body, world, &self.table, &mut self.cues);
        }
        let sources = (world, &self.table);
        self.ambient.update(
            seconds,
            listener.position,
            sources,
            &mut self.rng,
            &mut self.cues,
        );
        self.subtitles.update(seconds);
        for cue in std::mem::take(&mut self.cues) {
            self.play(cue);
        }
    }

    pub fn play(&mut self, cue: Cue) {
        let Some(sound) = self.bank.sound(cue.sound) else {
            return;
        };
        let (category, range) = (sound.def.category, sound.def.range);
        let subtitle = sound.def.subtitle.clone();
        let heard = self.listener.position.distance(cue.at) < f64::from(range);
        if let Some(key) = subtitle.filter(|_| self.settings.subtitles && cue.subtitled && heard) {
            let direction = Direction::of(&self.listener, cue.at);
            self.subtitles.heard(&key, direction);
        }
        if self.output.is_none() || self.settings.gain(category) <= 0.0 {
            return;
        }
        let Some(pick) = self.bank.pick(cue.sound, &mut self.rng) else {
            return;
        };
        self.send(Command::Play(Voice {
            buffer: pick.buffer,
            category,
            gain: 1.0,
            pitch: pick.pitch,
            at: Some(cue.at),
            range,
        }));
    }

    pub fn subtitles(&self) -> &[Subtitle] {
        self.subtitles.lines()
    }

    fn send(&self, command: Command) {
        if let Some(output) = &self.output {
            output.send(command);
        }
    }
}

/// The ears of a camera.
pub fn listener_at(position: DVec3, yaw: f32) -> Listener {
    Listener { position, yaw }
}
