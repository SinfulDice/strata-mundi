//! Which ambiences are heard, and how loud: the pack's ambiences weighted by the biomes
//! around and by how far underground the ears are, faded slowly from one to the next,
//! with their rare sounds played now and then around the listener.

use super::triggers::Cue;
use glam::DVec3;
use strata_core::audio::{Ambience, Rng, Sounds};
use strata_core::chunk::BiomeId;
use strata_core::content::SoundId;
use strata_core::id::Identifier;

/// Seconds for a bed to go from silence to full volume (or back).
pub const FADE_SECONDS: f32 = 3.0;
/// Loudest a bed may be asked, all its parts summed.
const MAX_BED_GAIN: f32 = 2.0;

/// The pack's ambiences, matched with the biomes the server names.
#[derive(Debug, Clone, Default)]
pub struct AmbienceTable {
    ambiences: Vec<Ambience>,
    /// The server's biome identifiers, by `BiomeId`, and the ambience of each.
    biomes: Vec<Identifier>,
    by_biome: Vec<Option<usize>>,
    underground: Option<usize>,
}

impl AmbienceTable {
    pub fn new(sounds: &Sounds) -> Self {
        let ambiences = sounds.ambiences.clone();
        let underground = ambiences.iter().position(|a| a.underground);
        Self {
            ambiences,
            underground,
            ..Self::default()
        }
    }

    /// Follows the server's list of biomes (cheap when it did not change).
    pub fn set_biomes<'a>(&mut self, biomes: impl Iterator<Item = &'a Identifier> + Clone) {
        if self.biomes.iter().eq(biomes.clone()) {
            return;
        }
        let biomes: Vec<&Identifier> = biomes.collect();
        self.by_biome = biomes
            .iter()
            .map(|id| self.ambiences.iter().position(|a| a.biomes.contains(id)))
            .collect();
        self.biomes = biomes.into_iter().cloned().collect();
    }

    pub fn of_biome(&self, biome: BiomeId) -> Option<usize> {
        self.by_biome.get(usize::from(biome.0)).copied().flatten()
    }

    pub fn underground(&self) -> Option<usize> {
        self.underground
    }

    pub fn get(&self, index: usize) -> Option<&Ambience> {
        self.ambiences.get(index)
    }

    pub fn count(&self) -> usize {
        self.ambiences.len()
    }
}

/// Where the ears are, as far as ambiences go.
#[derive(Debug, Clone, Default)]
pub struct Place {
    /// Each ambience heard and its share (the shares add up to 1 at most).
    pub weights: Vec<(usize, f32)>,
    pub head: DVec3,
    /// 0 at night, 1 by day.
    pub daylight: f32,
}

impl Place {
    /// `biomes`: the biomes around with their share; `underground` from 0 to 1.
    pub fn new(
        table: &AmbienceTable,
        biomes: &[(BiomeId, f32)],
        underground: f32,
        head: DVec3,
        daylight: f32,
    ) -> Self {
        let mut place = Self::default();
        place.refill(table, biomes, underground, head, daylight);
        place
    }

    /// The same as `new`, keeping the memory of the weights.
    pub fn refill(
        &mut self,
        table: &AmbienceTable,
        biomes: &[(BiomeId, f32)],
        underground: f32,
        head: DVec3,
        daylight: f32,
    ) {
        let underground = underground.clamp(0.0, 1.0);
        let weights = &mut self.weights;
        weights.clear();
        let mut add =
            |index: usize, weight: f32| match weights.iter_mut().find(|(known, _)| *known == index)
            {
                Some((_, total)) => *total += weight,
                None => weights.push((index, weight)),
            };
        for &(biome, share) in biomes {
            if let Some(index) = table.of_biome(biome) {
                add(index, share * (1.0 - underground));
            }
        }
        if let Some(index) = table.underground() {
            add(index, underground);
        }
        weights.retain(|(_, weight)| *weight > 0.0);
        self.head = head;
        self.daylight = daylight;
    }
}

/// Loudness of each bed sound wanted for `place`.
pub fn bed_targets(table: &AmbienceTable, place: &Place) -> Vec<(SoundId, f32)> {
    let mut targets = Vec::new();
    bed_targets_into(table, place, &mut targets);
    targets
}

fn bed_targets_into(table: &AmbienceTable, place: &Place, targets: &mut Vec<(SoundId, f32)>) {
    targets.clear();
    for &(index, weight) in &place.weights {
        let Some(ambience) = table.get(index) else {
            continue;
        };
        for bed in &ambience.beds {
            let height = bed
                .altitude
                .map_or(1.0, |altitude| altitude.factor(place.head.y as f32));
            let gain = weight * bed.volume * bed.time.factor(place.daylight) * height;
            match targets.iter_mut().find(|(sound, _)| *sound == bed.sound) {
                Some((_, total)) => *total += gain,
                None => targets.push((bed.sound, gain)),
            }
        }
    }
    for (_, gain) in targets.iter_mut() {
        *gain = gain.clamp(0.0, MAX_BED_GAIN);
    }
}

/// The beds' current loudness, gliding to their targets, and the events' countdowns.
#[derive(Debug, Default)]
pub struct AmbiencePlayer {
    levels: Vec<(SoundId, f32)>,
    targets: Vec<(SoundId, f32)>,
    /// Seconds before each event of each ambience; `None` until first heard.
    timers: Vec<Vec<Option<f32>>>,
}

impl AmbiencePlayer {
    pub fn levels(&self) -> &[(SoundId, f32)] {
        &self.levels
    }

    /// Glides the beds towards `place` (no faster than `FADE_SECONDS` from silence to
    /// full) and adds the events due to `cues`.
    pub fn update(
        &mut self,
        seconds: f32,
        table: &AmbienceTable,
        place: &Place,
        rng: &mut Rng,
        cues: &mut Vec<Cue>,
    ) {
        bed_targets_into(table, place, &mut self.targets);
        let targets = &self.targets;
        let step = seconds.max(0.0) / FADE_SECONDS;
        for &(sound, _) in targets {
            if !self.levels.iter().any(|(known, _)| *known == sound) {
                self.levels.push((sound, 0.0));
            }
        }
        for (sound, level) in &mut self.levels {
            let target = targets
                .iter()
                .find(|(wanted, _)| wanted == sound)
                .map_or(0.0, |(_, gain)| *gain);
            *level += (target - *level).clamp(-step, step);
        }
        self.events(seconds, table, place, rng, cues);
    }

    fn events(
        &mut self,
        seconds: f32,
        table: &AmbienceTable,
        place: &Place,
        rng: &mut Rng,
        cues: &mut Vec<Cue>,
    ) {
        self.timers.resize_with(table.count(), Vec::new);
        for &(index, weight) in &place.weights {
            let (Some(ambience), Some(timers)) = (table.get(index), self.timers.get_mut(index))
            else {
                continue;
            };
            timers.resize(ambience.events.len(), None);
            for (event, timer) in ambience.events.iter().zip(timers.iter_mut()) {
                let left = timer.get_or_insert_with(|| rng.range(event.every));
                *left -= seconds * weight * event.time.factor(place.daylight);
                if *left > 0.0 {
                    continue;
                }
                *left = rng.range(event.every);
                cues.push(Cue {
                    sound: event.sound,
                    at: around(place.head, event.distance, rng),
                    subtitled: true,
                });
            }
        }
    }
}

/// A random point at `distance` blocks around `head`, a little above or below.
fn around(head: DVec3, distance: (f32, f32), rng: &mut Rng) -> DVec3 {
    let angle = rng.unit() * std::f32::consts::TAU;
    let reach = rng.range(distance);
    let (sin, cos) = angle.sin_cos();
    head + DVec3::new(
        f64::from(cos * reach),
        f64::from(rng.signed() * 2.0),
        f64::from(sin * reach),
    )
}
