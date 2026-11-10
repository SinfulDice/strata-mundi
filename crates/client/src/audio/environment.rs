//! The place around the ears, followed over the frames: ambience beds and their events,
//! the kind of space (reverberation) and the muffling of the sounds still playing.

use super::ambience::{AmbiencePlayer, AmbienceTable, Place};
use super::mixer::Command;
use super::muffles::MuffleTracker;
use super::space::{Space, SpaceProbe};
use super::surroundings::{Surroundings, biomes_around, underground};
use super::triggers::Cue;
use glam::DVec3;
use strata_core::audio::{Rng, Sounds};
use strata_core::chunk::BiomeId;
use strata_core::content::SoundId;
use strata_protocol::BiomeInfo;

/// Seconds between two looks at the biomes around and the ground overhead.
const LOOK_EVERY: f32 = 0.25;
/// Smaller changes of a bed's loudness are not sent.
const NOTICEABLE: f32 = 1e-3;

#[derive(Default)]
pub struct Environment {
    table: AmbienceTable,
    player: AmbiencePlayer,
    probe: SpaceProbe,
    muffles: MuffleTracker,
    biomes: Vec<(BiomeId, f32)>,
    underground: f32,
    due: f32,
    space: Space,
    /// Loudness last sent for each bed.
    sent: Vec<(SoundId, f32)>,
}

/// What the ears are told this frame.
pub struct Moment<'a> {
    pub seconds: f32,
    pub head: DVec3,
    pub biomes: &'a [BiomeInfo],
    pub daylight: f32,
    /// Advanced spatial audio: the space is probed and the paths checked.
    pub spatial: bool,
}

impl Environment {
    pub fn new(sounds: &Sounds) -> Self {
        Self {
            table: AmbienceTable::new(sounds),
            ..Self::default()
        }
    }

    /// New content: the beds are sent again with their new buffers.
    pub fn set_content(&mut self, sounds: &Sounds) {
        self.table = AmbienceTable::new(sounds);
        self.sent.clear();
    }

    pub fn underground(&self) -> f32 {
        self.underground
    }

    pub fn space(&self) -> Space {
        self.space
    }

    /// Follows the place; ambience events go to `cues`, mixer commands to `send`.
    pub fn update(
        &mut self,
        moment: &Moment<'_>,
        world: &impl Surroundings,
        rng: &mut Rng,
        cues: &mut Vec<Cue>,
        send: &mut impl FnMut(Command),
    ) {
        self.table
            .set_biomes(moment.biomes.iter().map(|biome| &biome.id));
        self.due -= moment.seconds;
        if self.due <= 0.0 {
            self.due = LOOK_EVERY;
            self.biomes = biomes_around(world, moment.head);
            self.underground = underground(world, moment.head);
        }
        let place = Place::new(
            &self.table,
            &self.biomes,
            self.underground,
            moment.head,
            moment.daylight,
        );
        self.player
            .update(moment.seconds, &self.table, &place, rng, cues);
        if !moment.spatial {
            self.muffles.clear();
            self.space = Space::Outdoors;
            return;
        }
        let space = self.probe.update(world, moment.head, self.underground);
        if space != self.space {
            self.space = space;
            send(Command::Space(space));
        }
        self.muffles
            .update(moment.seconds, world, moment.head, send);
    }

    /// A sound started at `at` for `seconds`, muffled by `muffle`.
    pub fn track(&mut self, id: u32, at: DVec3, seconds: f32, muffle: f32) {
        self.muffles.add(id, at, seconds, muffle);
    }

    /// The beds whose loudness changed since last sent (now counted as sent).
    pub fn changed_beds(&mut self) -> Vec<(SoundId, f32)> {
        let mut changed = Vec::new();
        for &(sound, level) in self.player.levels() {
            let sent = match self.sent.iter_mut().find(|(known, _)| *known == sound) {
                Some((_, sent)) => sent,
                None => {
                    self.sent.push((sound, 0.0));
                    let last = self.sent.len() - 1;
                    &mut self.sent[last].1
                }
            };
            let fresh = (level - *sent).abs() >= NOTICEABLE || (level == 0.0 && *sent != 0.0);
            if fresh || (level > 0.0 && *sent == 0.0) {
                *sent = level;
                changed.push((sound, level));
            }
        }
        changed
    }

    pub fn levels(&self) -> &[(SoundId, f32)] {
        self.player.levels()
    }
}
