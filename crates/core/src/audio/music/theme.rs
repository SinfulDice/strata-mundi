//! Musical themes as written in `music/<name>.ron`: the mode, tempo, instruments, motifs
//! and density a piece is composed from. A pack may replace the pieces of a theme with
//! recordings in `music/<name>/*.ogg`.

use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Longest piece a theme may ask for, in seconds.
pub const MAX_PIECE_SECONDS: f32 = 240.0;
/// Most pieces (seeds) of one theme.
pub const MAX_PIECES: u8 = 16;
pub const MAX_MOTIFS: usize = 16;
pub const MAX_MOTIF_NOTES: usize = 16;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Theme", deny_unknown_fields)]
pub struct ThemeDef {
    /// Dimensions where it is heard.
    #[serde(default)]
    pub dimensions: Vec<Identifier>,
    /// Played on the title screen.
    #[serde(default)]
    pub title: bool,
    /// MIDI number of the tonic (60: middle C).
    pub root: u8,
    pub scale: Scale,
    /// Beats per minute.
    pub tempo: f32,
    /// Pieces composed from different seeds.
    #[serde(default = "default_pieces")]
    pub pieces: u8,
    /// Seconds a piece lasts (picked between the two).
    pub length: (f32, f32),
    /// 0 (a few notes) to 1 (phrases close together, more chords).
    pub density: f32,
    /// Seconds of silence between two phrases.
    pub rests: (f32, f32),
    /// Scale degrees the chords are built on (0: the tonic).
    pub chords: Vec<i8>,
    pub motifs: Vec<Motif>,
    pub piano: Piano,
    #[serde(default)]
    pub pad: Option<Pad>,
    #[serde(default)]
    pub room: Room,
    /// Loudness of the whole piece, 0 to 1.
    #[serde(default = "default_volume")]
    pub volume: f32,
}

/// Steps of a mode, in semitones above the tonic.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Scale {
    Major,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Minor,
    PentatonicMajor,
    PentatonicMinor,
    /// Semitones 0 to 11, the tonic first.
    Custom(Vec<u8>),
}

impl Scale {
    pub fn steps(&self) -> &[u8] {
        match self {
            Self::Major => &[0, 2, 4, 5, 7, 9, 11],
            Self::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Self::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Self::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Self::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Self::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Self::PentatonicMajor => &[0, 2, 4, 7, 9],
            Self::PentatonicMinor => &[0, 3, 5, 7, 10],
            Self::Custom(steps) => steps,
        }
    }

    /// MIDI note of `degree` above `root` (degrees past the scale go up an octave).
    pub fn note(&self, root: i32, degree: i32) -> u8 {
        let steps = self.steps();
        let len = steps.len().max(1) as i32;
        let octave = degree.div_euclid(len);
        let step = steps.get(degree.rem_euclid(len) as usize).copied();
        let note = root + octave * 12 + i32::from(step.unwrap_or(0));
        note.clamp(0, 127) as u8
    }
}

/// A short melodic cell: scale degrees and their lengths in beats.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Motif", deny_unknown_fields)]
pub struct Motif {
    pub degrees: Vec<i8>,
    pub beats: Vec<f32>,
}

/// The synthesised piano: a hammer, a few slightly stretched harmonics, a long decay.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Piano", deny_unknown_fields)]
pub struct Piano {
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Seconds a middle C takes to fade by 60 dB (lower notes ring longer).
    pub decay: f32,
    /// 0 (soft, round) to 1 (bright).
    #[serde(default = "half")]
    pub brightness: f32,
    /// Octaves of the melody above the tonic.
    #[serde(default)]
    pub octave: i8,
    /// Chance of a low note under each phrase.
    #[serde(default = "half")]
    pub bass: f32,
}

/// Soft sustained chords.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Pad", deny_unknown_fields)]
pub struct Pad {
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Seconds to swell in and to fade out.
    pub attack: f32,
    pub release: f32,
    /// Low-pass cutoff of the pad, in hertz.
    #[serde(default = "pad_cutoff")]
    pub cutoff: f32,
    #[serde(default = "minus_one")]
    pub octave: i8,
    /// Chance that a phrase is held by a chord.
    #[serde(default = "half")]
    pub chance: f32,
}

/// The reverberation the piece is rendered with.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Room {
    /// 0 (small) to 1 (a hall).
    pub size: f32,
    /// 0 (dry) to 1.
    pub wet: f32,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            size: 0.6,
            wet: 0.25,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub id: Identifier,
    pub def: ThemeDef,
    /// Recordings that replace the composed pieces (decoded by the client).
    pub recordings: Vec<PathBuf>,
}

impl Theme {
    pub fn heard_in(&self, dimension: &Identifier) -> bool {
        self.def.dimensions.contains(dimension)
    }
}

pub(in crate::audio) fn load_themes(packs: &PackSet, errors: &mut Vec<PackError>) -> Vec<Theme> {
    let recordings = recordings(packs, errors);
    let mut themes: Vec<Theme> = packs
        .layered::<ThemeDef>("music", errors)
        .into_iter()
        .map(|(id, Sourced { mut value, source })| {
            let mut problems = Vec::new();
            bound(&mut value, &mut problems);
            for problem in problems {
                errors.push(PackError::invalid(&source, problem));
            }
            Theme {
                id,
                def: value,
                recordings: Vec::new(),
            }
        })
        .collect();
    for (id, folder, files) in recordings {
        match themes.iter_mut().find(|theme| theme.id == id) {
            Some(theme) => theme.recordings = files,
            None => {
                let message = format!("no `music/{}.ron` defines `{id}`", id.name());
                errors.push(PackError::invalid(&folder, message));
            }
        }
    }
    themes
}

/// `music/<theme>/*.ogg` (the pack's namespace) or `music/<namespace>/<theme>/*.ogg`; a
/// higher pack's folder replaces a lower one's.
fn recordings(
    packs: &PackSet,
    errors: &mut Vec<PackError>,
) -> Vec<(Identifier, PathBuf, Vec<PathBuf>)> {
    let mut found: Vec<(Identifier, PathBuf, Vec<PathBuf>)> = Vec::new();
    for pack in packs.packs() {
        let music = pack.root.join("music");
        for folder in folders(&music) {
            let name = stem(&folder);
            let mut add = |namespace: &str, name: &str, folder: &Path| {
                let files = oggs(folder);
                if files.is_empty() {
                    return;
                }
                match Identifier::new(namespace, name) {
                    Ok(id) => {
                        found.retain(|(other, ..)| *other != id);
                        found.push((id, folder.to_owned(), files));
                    }
                    Err(error) => errors.push(PackError::invalid(folder, error.to_string())),
                }
            };
            add(&pack.manifest.namespace, &name, &folder);
            for inner in folders(&folder) {
                add(&name, &stem(&inner), &inner);
            }
        }
    }
    found
}

fn folders(dir: &Path) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    list.sort();
    list
}

fn oggs(dir: &Path) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "ogg"))
        .collect();
    list.sort();
    list
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub(super) fn bound(def: &mut ThemeDef, problems: &mut Vec<String>) {
    let mut value = |name: &str, value: &mut f32, min: f32, max: f32| {
        if !(value.is_finite() && (min..=max).contains(value)) {
            problems.push(format!("`{name}` must be between {min} and {max}"));
            *value = if value.is_finite() {
                value.clamp(min, max)
            } else {
                min
            };
        }
    };
    value("tempo", &mut def.tempo, 20.0, 240.0);
    value("density", &mut def.density, 0.0, 1.0);
    value("volume", &mut def.volume, 0.0, 1.0);
    value("length.0", &mut def.length.0, 10.0, MAX_PIECE_SECONDS);
    value("length.1", &mut def.length.1, 10.0, MAX_PIECE_SECONDS);
    value("rests.0", &mut def.rests.0, 0.5, 60.0);
    value("rests.1", &mut def.rests.1, 0.5, 60.0);
    value("piano.volume", &mut def.piano.volume, 0.0, 1.0);
    value("piano.decay", &mut def.piano.decay, 0.2, 20.0);
    value("piano.brightness", &mut def.piano.brightness, 0.0, 1.0);
    value("piano.bass", &mut def.piano.bass, 0.0, 1.0);
    value("room.size", &mut def.room.size, 0.0, 1.0);
    value("room.wet", &mut def.room.wet, 0.0, 1.0);
    if let Some(pad) = &mut def.pad {
        value("pad.volume", &mut pad.volume, 0.0, 1.0);
        value("pad.attack", &mut pad.attack, 0.05, 20.0);
        value("pad.release", &mut pad.release, 0.05, 20.0);
        value("pad.cutoff", &mut pad.cutoff, 100.0, 8000.0);
        value("pad.chance", &mut pad.chance, 0.0, 1.0);
    }
    order(&mut def.length);
    order(&mut def.rests);
    if def.pieces == 0 || def.pieces > MAX_PIECES {
        problems.push(format!("`pieces` must be between 1 and {MAX_PIECES}"));
        def.pieces = def.pieces.clamp(1, MAX_PIECES);
    }
    bound_notes(def, problems);
}

fn bound_notes(def: &mut ThemeDef, problems: &mut Vec<String>) {
    let steps = def.scale.steps();
    let custom_ok = !steps.is_empty() && steps[0] == 0 && steps.iter().all(|s| *s < 12);
    if !custom_ok {
        problems.push("a scale needs steps 0 to 11, starting with 0".to_owned());
        def.scale = Scale::Major;
    }
    if def.root < 24 || def.root > 96 {
        problems.push("`root` must be a MIDI note between 24 and 96".to_owned());
        def.root = def.root.clamp(24, 96);
    }
    if def.chords.is_empty() {
        problems.push("a theme needs at least one chord".to_owned());
        def.chords.push(0);
    }
    if def.motifs.is_empty() || def.motifs.len() > MAX_MOTIFS {
        problems.push(format!("a theme needs 1 to {MAX_MOTIFS} motifs"));
        def.motifs.truncate(MAX_MOTIFS);
    }
    for motif in &mut def.motifs {
        let len = motif.degrees.len();
        if len == 0 || len > MAX_MOTIF_NOTES || motif.beats.len() != len {
            problems.push(format!(
                "a motif needs 1 to {MAX_MOTIF_NOTES} degrees and as many beats"
            ));
            motif.degrees.truncate(MAX_MOTIF_NOTES);
            motif.beats.resize(motif.degrees.len(), 1.0);
        }
        for beat in &mut motif.beats {
            if !(beat.is_finite() && (0.125..=16.0).contains(beat)) {
                problems.push("motif beats must be between 0.125 and 16".to_owned());
                *beat = 1.0;
            }
        }
    }
    def.motifs.retain(|motif| !motif.degrees.is_empty());
    if def.motifs.is_empty() {
        def.motifs.push(Motif {
            degrees: vec![0],
            beats: vec![1.0],
        });
    }
}

fn order(pair: &mut (f32, f32)) {
    if pair.0 > pair.1 {
        *pair = (pair.1, pair.0);
    }
}

fn default_pieces() -> u8 {
    4
}

fn default_volume() -> f32 {
    0.8
}

fn half() -> f32 {
    0.5
}

fn pad_cutoff() -> f32 {
    1200.0
}

fn minus_one() -> i8 {
    -1
}
