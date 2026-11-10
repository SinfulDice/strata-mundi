//! Ambiences as written in `ambiences/<name>.ron`: the looping beds and the rare sounds
//! of a group of biomes, or of the underground. Biomes stay identifiers here: the client
//! matches them with the biomes the server names.

use super::load::Sound;
use crate::content::{Registry, SoundId};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Ambience", deny_unknown_fields)]
pub struct AmbienceDef {
    /// Biomes where it is heard (above ground).
    #[serde(default)]
    pub biomes: Vec<Identifier>,
    /// Heard underground instead, whatever the biome.
    #[serde(default)]
    pub underground: bool,
    #[serde(default)]
    pub beds: Vec<BedDef>,
    #[serde(default)]
    pub events: Vec<EventDef>,
}

/// A sound looping as long as the ambience is heard.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BedDef {
    pub sound: Identifier,
    #[serde(default = "one")]
    pub volume: f32,
    #[serde(default)]
    pub time: TimeOfDay,
    /// Louder (or quieter) with the listener's height.
    #[serde(default)]
    pub altitude: Option<Altitude>,
}

/// A sound played now and then somewhere around the listener.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDef {
    pub sound: Identifier,
    /// Seconds between two plays.
    pub every: (f32, f32),
    /// Blocks from the listener.
    #[serde(default = "default_distance")]
    pub distance: (f32, f32),
    #[serde(default)]
    pub time: TimeOfDay,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub enum TimeOfDay {
    #[default]
    Any,
    Day,
    Night,
}

impl TimeOfDay {
    /// How much it is heard for `daylight` (0 at night, 1 by day).
    pub fn factor(self, daylight: f32) -> f32 {
        let daylight = daylight.clamp(0.0, 1.0);
        match self {
            Self::Any => 1.0,
            Self::Day => daylight,
            Self::Night => 1.0 - daylight,
        }
    }
}

/// Gain `gain.0` at height `from` and below, `gain.1` at `to` and above.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Altitude {
    pub from: f32,
    pub to: f32,
    pub gain: (f32, f32),
}

impl Altitude {
    pub fn factor(&self, height: f32) -> f32 {
        let span = (self.to - self.from).max(1.0);
        let t = ((height - self.from) / span).clamp(0.0, 1.0);
        self.gain.0 + (self.gain.1 - self.gain.0) * t
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ambience {
    pub id: Identifier,
    pub biomes: Vec<Identifier>,
    pub underground: bool,
    pub beds: Vec<Bed>,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bed {
    pub sound: SoundId,
    pub volume: f32,
    pub time: TimeOfDay,
    pub altitude: Option<Altitude>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    pub sound: SoundId,
    pub every: (f32, f32),
    pub distance: (f32, f32),
    pub time: TimeOfDay,
}

/// Most beds and events of one ambience.
pub const MAX_PARTS: usize = 8;

pub(super) fn load_ambiences(
    packs: &PackSet,
    sounds: &Registry<SoundId, Sound>,
    errors: &mut Vec<PackError>,
) -> Vec<Ambience> {
    packs
        .layered::<AmbienceDef>("ambiences", errors)
        .into_iter()
        .map(|(id, Sourced { value, source })| resolve(id, value, sounds, &source, errors))
        .collect()
}

fn resolve(
    id: Identifier,
    def: AmbienceDef,
    sounds: &Registry<SoundId, Sound>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Ambience {
    let mut problem = |message: String| errors.push(PackError::invalid(source, message));
    if def.underground == !def.biomes.is_empty() {
        problem("an ambience needs `biomes` or `underground: true`, not both".to_owned());
    }
    if def.beds.len() > MAX_PARTS || def.events.len() > MAX_PARTS {
        problem(format!("at most {MAX_PARTS} beds and {MAX_PARTS} events"));
    }
    let mut find = |sound: &Identifier| {
        let key = sounds.key(sound);
        if key.is_none() {
            problem(format!("unknown sound `{sound}`"));
        }
        key
    };
    let beds = def
        .beds
        .iter()
        .take(MAX_PARTS)
        .filter_map(|bed| {
            Some(Bed {
                sound: find(&bed.sound)?,
                volume: bounded(bed.volume, 0.0, 2.0),
                time: bed.time,
                altitude: bed.altitude,
            })
        })
        .collect();
    let events = def
        .events
        .iter()
        .take(MAX_PARTS)
        .filter_map(|event| {
            Some(Event {
                sound: find(&event.sound)?,
                every: ordered(event.every, 0.5, 3600.0),
                distance: ordered(event.distance, 0.0, 64.0),
                time: event.time,
            })
        })
        .collect();
    Ambience {
        id,
        biomes: def.biomes,
        underground: def.underground,
        beds,
        events,
    }
}

fn bounded(value: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

fn ordered((a, b): (f32, f32), min: f32, max: f32) -> (f32, f32) {
    let (a, b) = (bounded(a, min, max), bounded(b, min, max));
    (a.min(b), a.max(b))
}

fn one() -> f32 {
    1.0
}

fn default_distance() -> (f32, f32) {
    (4.0, 12.0)
}
