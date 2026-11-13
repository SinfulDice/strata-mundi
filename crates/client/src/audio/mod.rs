//! Sound: synthesised (or recorded) buffers played in 3D around the listener, ambiences
//! of the place, reverberation and muffling (advanced spatial audio), rare music of the
//! place, volumes by category, subtitles. Without an audio device everything runs the
//! same, silently (and no music is rendered).

mod ambience;
#[cfg(test)]
mod ambience_tests;
mod bank;
#[cfg(test)]
mod bank_tests;
mod beds;
mod controls;
mod decode;
mod environment;
mod mixer;
mod muffles;
mod music;
#[cfg(test)]
mod music_tests;
mod music_threads;
mod output;
#[cfg(test)]
mod place_tests;
mod reverb;
mod space;
mod spatial;
mod stream;
mod subtitles;
mod surroundings;
#[cfg(test)]
mod tests;
mod trash;
mod triggers;

pub use ambience::{AmbiencePlayer, AmbienceTable, FADE_SECONDS, Place, bed_targets};
pub use beds::{BedVolume, MAX_BEDS};
pub use environment::{Environment, Moment};
pub use mixer::{Command, MAX_VOICES, Mixer, Voice, Volumes};
pub use music::{MusicMoment, MusicPlace, MusicPlayer, MusicSchedule, TITLE_WAIT};
pub use reverb::{Reverb, Room, Rooms};
pub use space::{REACH, Space, SpaceProbe};
pub use spatial::{Direction, Listener, attenuation, stereo_gains};
pub use stream::{MUSIC_FADE, MusicCommand, MusicFeed};
pub use subtitles::{LIFETIME, MAX_LINES, Subtitle, Subtitles};
pub use surroundings::{Surroundings, WALL, WorldView, biomes_around, muffling, underground};
pub use triggers::{AmbientBlocks, BlockSounds, Cue, Footsteps, SoundTable};

use crate::settings::SoundSettings;
use crate::world::ClientWorld;
use bank::SoundBank;
use glam::DVec3;
use output::Output;
use std::path::PathBuf;
use strata_core::audio::{Rng, SAMPLE_RATE, SoundCategory};
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::physics::Body;
use strata_protocol::BiomeInfo;

/// Sounds started at most in one frame.
pub const MAX_STARTS: usize = 16;
/// The same sound started this close in one frame is heard once.
const GROUP_BLOCKS: f64 = 4.0;
/// A sound whose buffer was not made yet is started when it is, if it waited less than
/// this (later it would not match what is seen).
pub const MAX_LATE: f32 = 0.1;

/// A sound waiting for its buffer.
struct Late {
    cue: Cue,
    muffle: f32,
    waited: f32,
}

enum Start {
    Started,
    NotReady,
    Silent,
}

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
    /// The cues of this frame with their muffling (kept to reuse its memory).
    heard: Vec<(Cue, f32)>,
    late: Vec<Late>,
    next_voice: u32,
    music: MusicPlayer,
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
    pub place: MusicPlace,
    /// The game is paused (the music goes on, its silences are not counted).
    pub paused: bool,
}

impl Audio {
    /// Opens the default device; without one the game goes on in silence. Music is
    /// cached in `music_cache`.
    pub fn start(content: &Content, settings: &SoundSettings, music_cache: PathBuf) -> Self {
        let output = Output::open(Volumes::of(settings), settings.spatial.is_advanced());
        Self::with_output(output, content, settings, music_cache)
    }

    /// No device at all (captures, tests).
    pub fn silent(content: &Content, settings: &SoundSettings) -> Self {
        Self::with_output(None, content, settings, PathBuf::new())
    }

    fn with_output(
        output: Option<Output>,
        content: &Content,
        settings: &SoundSettings,
        music_cache: PathBuf,
    ) -> Self {
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
            heard: Vec::new(),
            late: Vec::new(),
            next_voice: 0,
            music: MusicPlayer::new(music_cache, content.sounds.themes.clone()),
        };
        audio.apply_settings(settings);
        audio.prepare_sounds();
        audio
    }

    /// With a device, every buffer is made at once in the background.
    fn prepare_sounds(&mut self) {
        if self.output.is_some() {
            self.bank.prepare_all();
        }
    }

    pub fn has_device(&self) -> bool {
        self.output.is_some()
    }

    /// New content (resources reloaded): buffers are made again from the new recipes.
    pub fn set_content(&mut self, content: &Content) {
        self.bank = SoundBank::new(content.sounds.clone());
        self.table = SoundTable::from_content(content);
        self.environment.set_content(&content.sounds);
        self.music.set_themes(content.sounds.themes.clone());
        self.late.clear();
        self.prepare_sounds();
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
            gather(&mut self.cues, cue);
        }
    }

    /// Moves the ears, plays the local player's steps, the blocks and the ambiences
    /// around, follows the place, and starts what was asked this frame.
    pub fn update(&mut self, seconds: f32, hearing: &Hearing<'_>) {
        let (listener, world) = (hearing.listener, hearing.world);
        if let Some(output) = &self.output {
            output.empty_trash();
        }
        self.bank.collect();
        self.start_late(seconds);
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
            &mut |command| output.is_some_and(|output| output.send(command)),
        );
        let mut heard = std::mem::take(&mut self.heard);
        for cue in self.cues.drain(..) {
            if !is_grouped(&heard, &cue) {
                let muffle = if spatial {
                    muffling(&view, cue.at, listener.position)
                } else {
                    0.0
                };
                heard.push((cue, muffle));
            }
        }
        self.send_beds();
        self.update_music(seconds, hearing);
        self.subtitles.update(seconds);
        for (cue, muffle) in heard.drain(..) {
            self.play_muffled(cue, muffle);
        }
        self.heard = heard;
    }

    pub fn play(&mut self, cue: Cue) {
        self.play_muffled(cue, 0.0);
    }

    fn play_muffled(&mut self, cue: Cue, muffle: f32) {
        let Some(sound) = self.bank.sound(cue.sound) else {
            return;
        };
        let range = sound.def.range;
        let heard = self.listener.position.distance(cue.at) < f64::from(range);
        if self.settings.subtitles
            && cue.subtitled
            && heard
            && let Some(key) = &sound.def.subtitle
        {
            let direction = Direction::of(&self.listener, cue.at);
            self.subtitles.heard(key, direction);
        }
        let start = self.start_voice(cue, muffle);
        if matches!(start, Start::NotReady) && self.late.len() < MAX_STARTS {
            self.late.push(Late {
                cue,
                muffle,
                waited: 0.0,
            });
        }
    }

    /// Starts the sounds whose buffer came in time, forgets those too late.
    fn start_late(&mut self, seconds: f32) {
        let mut late = std::mem::take(&mut self.late);
        late.retain_mut(|waiting| {
            waiting.waited += seconds;
            waiting.waited <= MAX_LATE
                && matches!(
                    self.start_voice(waiting.cue, waiting.muffle),
                    Start::NotReady
                )
        });
        self.late = late;
    }

    fn start_voice(&mut self, cue: Cue, muffle: f32) -> Start {
        let Some(sound) = self.bank.sound(cue.sound) else {
            return Start::Silent;
        };
        let (category, range) = (sound.def.category, sound.def.range);
        if self.output.is_none() || self.settings.gain(category) <= 0.0 {
            return Start::Silent;
        }
        let Some(pick) = self.bank.pick(cue.sound, &mut self.rng) else {
            return if self.bank.is_coming(cue.sound) {
                Start::NotReady
            } else {
                Start::Silent
            };
        };
        self.next_voice = self.next_voice.wrapping_add(1);
        let seconds = pick.buffer.len() as f32 / SAMPLE_RATE as f32 / pick.pitch.max(0.01);
        let sent = self.send(Command::Play(Voice {
            id: self.next_voice,
            buffer: pick.buffer,
            category,
            gain: 1.0,
            pitch: pick.pitch,
            at: Some(cue.at),
            range,
            muffle,
        }));
        if sent {
            self.environment
                .track(self.next_voice, cue.at, seconds, muffle);
        }
        Start::Started
    }

    /// Starts, fades or lets go the ambience beds (buffers made on first use).
    fn send_beds(&mut self) {
        let Some(output) = &self.output else {
            return;
        };
        let bank = &mut self.bank;
        self.environment.send_beds(&mut |sound, gain| {
            let category = bank
                .sound(sound)
                .map_or(SoundCategory::Ambient, |s| s.def.category);
            let Some(buffer) = bank.bed(sound) else {
                return false;
            };
            output.send(Command::Bed(BedVolume {
                sound,
                buffer,
                category,
                gain,
            }))
        });
    }

    fn update_music(&mut self, seconds: f32, hearing: &Hearing<'_>) {
        let moment = MusicMoment {
            seconds,
            place: &hearing.place,
            paused: hearing.paused,
            enabled: self.output.is_some() && self.settings.gain(SoundCategory::Music) > 0.0,
            frequency: self.settings.music_frequency,
        };
        let output = self.output.as_ref();
        self.music.update(&moment, &mut |command| {
            output.is_some_and(|output| output.send(command))
        });
    }

    pub fn subtitles(&self) -> &[Subtitle] {
        self.subtitles.lines()
    }

    fn send(&self, command: Command) -> bool {
        self.output
            .as_ref()
            .is_some_and(|output| output.send(command))
    }
}

/// Keeps `cue` unless it is grouped with one already there (a `/fill` or a big break
/// would start thousands of sounds at once).
fn gather(cues: &mut Vec<Cue>, cue: Cue) {
    if !cues.iter().any(|known| same_sound_near(known, &cue)) && cues.len() < MAX_STARTS {
        cues.push(cue);
    }
}

fn is_grouped(heard: &[(Cue, f32)], cue: &Cue) -> bool {
    heard.len() >= MAX_STARTS || heard.iter().any(|(known, _)| same_sound_near(known, cue))
}

fn same_sound_near(a: &Cue, b: &Cue) -> bool {
    a.sound == b.sound && a.at.distance_squared(b.at) < GROUP_BLOCKS * GROUP_BLOCKS
}

/// The ears of a camera.
pub fn listener_at(position: DVec3, yaw: f32) -> Listener {
    Listener { position, yaw }
}
