//! Sound: synthesised (or recorded) buffers played in 3D around the listener, ambiences
//! of the place, reverberation and muffling (advanced spatial audio), volumes by
//! category, subtitles. Without an audio device everything runs the same, silently.

mod ambience;
mod bank;
mod beds;
mod environment;
mod mixer;
mod muffles;
mod output;
#[cfg(test)]
mod place_tests;
mod reverb;
mod space;
mod spatial;
mod subtitles;
mod surroundings;
#[cfg(test)]
mod tests;
mod triggers;

pub use ambience::{AmbiencePlayer, AmbienceTable, FADE_SECONDS, Place, bed_targets};
pub use beds::{BedVolume, MAX_BEDS};
pub use environment::{Environment, Moment};
pub use mixer::{Command, MAX_VOICES, Mixer, Voice, Volumes};
pub use reverb::{Reverb, Room};
pub use space::{REACH, Space, SpaceProbe};
pub use spatial::{Direction, Listener, attenuation, stereo_gains};
pub use subtitles::{LIFETIME, MAX_LINES, Subtitle, Subtitles};
pub use surroundings::{Surroundings, WALL, WorldView, biomes_around, muffling, underground};
pub use triggers::{AmbientBlocks, BlockSounds, Cue, Footsteps, SoundTable};

use crate::settings::SoundSettings;
use crate::world::ClientWorld;
use bank::SoundBank;
use glam::DVec3;
use output::Output;
use strata_core::audio::{Rng, SAMPLE_RATE, SoundCategory};
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::physics::Body;
use strata_protocol::BiomeInfo;

pub struct Audio {
    output: Option<Output>,
    bank: SoundBank,
    table: SoundTable,
    settings: SoundSettings,
    listener: Listener,
    subtitles: Subtitles,
    footsteps: Footsteps,
    ambient: AmbientBlocks,
    environment: Environment,
    rng: Rng,
    cues: Vec<Cue>,
    next_voice: u32,
}

/// What is heard this frame and where from.
pub struct Hearing<'a> {
    pub listener: Listener,
    /// `None` on the title screen or in a boat.
    pub player: Option<&'a Body>,
    pub world: &'a ClientWorld,
    /// The server's biomes, by `BiomeId` (empty on the title screen).
    pub biomes: &'a [BiomeInfo],
    /// 0 at night, 1 by day.
    pub daylight: f32,
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
        let mut audio = Self {
            output,
            bank: SoundBank::new(content.sounds.clone()),
            table: SoundTable::from_content(content),
            settings: settings.clone(),
            listener: Listener::default(),
            subtitles: Subtitles::default(),
            footsteps: Footsteps::default(),
            ambient: AmbientBlocks::default(),
            environment: Environment::new(&content.sounds),
            rng: Rng::new(0x50_17D5),
            cues: Vec::new(),
            next_voice: 0,
        };
        audio.apply_settings(settings);
        audio
    }

    pub fn has_device(&self) -> bool {
        self.output.is_some()
    }

    /// New content (resources reloaded): buffers are made again from the new recipes.
    pub fn set_content(&mut self, content: &Content) {
        self.bank = SoundBank::new(content.sounds.clone());
        self.table = SoundTable::from_content(content);
        self.environment.set_content(&content.sounds);
    }

    pub fn apply_settings(&mut self, settings: &SoundSettings) {
        if !settings.subtitles {
            self.subtitles.clear();
        }
        self.settings = settings.clone();
        self.send(Command::Volumes(Volumes::of(settings)));
        self.send(Command::Spatial(settings.spatial.is_advanced()));
    }

    /// A block changed (server decision): it may have been broken or placed.
    pub fn block_changed(&mut self, pos: BlockPos, old: BlockId, new: BlockId) {
        if let Some(cue) = self.table.changed(pos, old, new) {
            self.cues.push(cue);
        }
    }

    /// Moves the ears, plays the local player's steps, the blocks and the ambiences
    /// around, follows the place, and starts what was asked this frame.
    pub fn update(&mut self, seconds: f32, hearing: &Hearing<'_>) {
        let (listener, world) = (hearing.listener, hearing.world);
        self.listener = listener;
        self.send(Command::Listener(listener));
        if let Some(body) = hearing.player {
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
        let view = WorldView {
            world,
            table: &self.table,
        };
        let spatial = self.settings.spatial.is_advanced();
        let moment = Moment {
            seconds,
            head: listener.position,
            biomes: hearing.biomes,
            daylight: hearing.daylight,
            spatial,
        };
        let output = self.output.as_ref();
        self.environment.update(
            &moment,
            &view,
            &mut self.rng,
            &mut self.cues,
            &mut |command| {
                if let Some(output) = output {
                    output.send(command);
                }
            },
        );
        let heard: Vec<(Cue, f32)> = std::mem::take(&mut self.cues)
            .into_iter()
            .map(|cue| {
                let muffle = if spatial {
                    muffling(&view, cue.at, listener.position)
                } else {
                    0.0
                };
                (cue, muffle)
            })
            .collect();
        self.send_beds();
        self.subtitles.update(seconds);
        for (cue, muffle) in heard {
            self.play_muffled(cue, muffle);
        }
    }

    pub fn play(&mut self, cue: Cue) {
        self.play_muffled(cue, 0.0);
    }

    fn play_muffled(&mut self, cue: Cue, muffle: f32) {
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
        self.next_voice = self.next_voice.wrapping_add(1);
        let seconds = pick.buffer.len() as f32 / SAMPLE_RATE as f32 / pick.pitch.max(0.01);
        self.environment
            .track(self.next_voice, cue.at, seconds, muffle);
        self.send(Command::Play(Voice {
            id: self.next_voice,
            buffer: pick.buffer,
            category,
            gain: 1.0,
            pitch: pick.pitch,
            at: Some(cue.at),
            range,
            muffle,
        }));
    }

    /// Starts, fades or lets go the ambience beds (buffers made on first use).
    fn send_beds(&mut self) {
        if self.output.is_none() {
            return;
        }
        for (sound, gain) in self.environment.changed_beds() {
            let category = self
                .bank
                .sound(sound)
                .map_or(SoundCategory::Ambient, |s| s.def.category);
            let Some(buffer) = self.bank.bed(sound) else {
                continue;
            };
            self.send(Command::Bed(BedVolume {
                sound,
                buffer,
                category,
                gain,
            }));
        }
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
