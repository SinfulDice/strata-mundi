//! The music cache: each piece is rendered once into a WAV file of the game's data
//! folder, then only read. The file name holds everything the sound depends on (theme,
//! piece, mood, version of the algorithm, the theme's settings), so a change makes a new
//! file and the old one is pruned.

use super::compose::{Mood, compose};
use super::render::PieceRenderer;
use super::theme::{Theme, ThemeDef};
use crate::audio::SAMPLE_RATE;
use crate::audio::load::seed_of;
use crate::audio::wav::{WAV_HEADER, pcm16, wav_header};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Bumped whenever composition or synthesis changes: every cached piece is made again.
pub const ALGORITHM: u32 = 1;
const BLOCK: usize = 4096;
const EXTENSION: &str = "wav";

/// One piece of a theme.
#[derive(Debug, Clone, Copy)]
pub struct Piece<'a> {
    pub theme: &'a Theme,
    pub index: u8,
    pub mood: Mood,
}

impl Piece<'_> {
    pub fn seed(&self) -> u64 {
        seed_of(&self.theme.id) ^ (u64::from(self.index) + 1).wrapping_mul(0xD1B5_4A32_D192_ED03)
    }

    /// `<namespace>-<theme>-<mood>-<piece>-a<algorithm>-<settings hash>.wav`
    pub fn file_name(&self) -> String {
        format!(
            "{}.{EXTENSION}",
            stem(self.theme, self.mood, self.index, ALGORITHM)
        )
    }

    /// Composes and renders the whole piece in memory (tests and tools).
    pub fn render(&self) -> Vec<f32> {
        let def = &self.theme.def;
        super::render::render_piece(def, compose(def, self.seed(), self.mood), self.seed())
    }
}

fn prefix(theme: &Theme) -> String {
    format!("{}-{}-", theme.id.namespace(), theme.id.name())
}

fn stem(theme: &Theme, mood: Mood, index: u8, algorithm: u32) -> String {
    format!(
        "{}{}-{index}-a{algorithm}-{:016x}",
        prefix(theme),
        mood.id(),
        settings_hash(&theme.def)
    )
}

/// FNV-1a of the theme as written: any change of a value gives other pieces.
fn settings_hash(def: &ThemeDef) -> u64 {
    format!("{def:?}")
        .bytes()
        .fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cached {
    pub path: PathBuf,
    /// Made now rather than found in the cache.
    pub rendered: bool,
}

/// The cached file of `piece`, rendered first if missing. Written to a `.part` file then
/// renamed, so a crash never leaves half a piece under the real name.
pub fn cached_piece(dir: &Path, piece: Piece<'_>) -> io::Result<Cached> {
    let path = dir.join(piece.file_name());
    if path.is_file() {
        return Ok(Cached {
            path,
            rendered: false,
        });
    }
    std::fs::create_dir_all(dir)?;
    let part = path.with_extension("part");
    write_piece(&part, piece)?;
    std::fs::rename(&part, &path)?;
    Ok(Cached {
        path,
        rendered: true,
    })
}

fn write_piece(path: &Path, piece: Piece<'_>) -> io::Result<()> {
    let def = &piece.theme.def;
    let seed = piece.seed();
    let mut renderer = PieceRenderer::new(def, compose(def, seed, piece.mood), seed);
    let mut file = BufWriter::new(File::create(path)?);
    file.write_all(&wav_header(renderer.frames()))?;
    let mut block = vec![0.0; BLOCK];
    let mut bytes = Vec::with_capacity(BLOCK * 2);
    loop {
        let count = renderer.fill(&mut block);
        if count == 0 {
            break;
        }
        bytes.clear();
        pcm16(&block[..count], &mut bytes);
        file.write_all(&bytes)?;
    }
    file.into_inner()
        .map_err(io::IntoInnerError::into_error)?
        .sync_all()
}

/// Removes the files of `themes` that no current piece uses (older algorithm, changed
/// settings, unfinished renders). Files of other themes are left alone.
pub fn prune(dir: &Path, themes: &[Theme]) -> io::Result<usize> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(0);
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(theme) = themes.iter().find(|t| name.starts_with(&prefix(t))) else {
            continue;
        };
        let current = (0..theme.def.pieces).any(|index| {
            let piece = Piece {
                theme,
                index,
                mood: Mood::Calm,
            };
            piece.file_name() == name
        });
        if !current {
            std::fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Reads a cached piece a block at a time.
pub struct WavStream {
    reader: BufReader<File>,
    left: usize,
    frames: usize,
    bytes: Vec<u8>,
}

impl WavStream {
    /// Only the files the cache writes: 16-bit mono at `SAMPLE_RATE`.
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut header = [0; WAV_HEADER];
        reader.read_exact(&mut header)?;
        let data_len = u32::from_le_bytes([header[40], header[41], header[42], header[43]]);
        let frames = data_len as usize / 2;
        if header != wav_header(frames) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a 16-bit mono WAV at 44.1 kHz",
            ));
        }
        Ok(Self {
            reader,
            left: frames,
            frames,
            bytes: Vec::new(),
        })
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn seconds(&self) -> f32 {
        self.frames as f32 / SAMPLE_RATE as f32
    }

    /// Up to `max` next samples (fewer at the end, none once finished).
    pub fn read(&mut self, max: usize) -> io::Result<Vec<f32>> {
        let mut samples = Vec::new();
        self.read_into(&mut samples, max)?;
        Ok(samples)
    }

    /// Replaces the content of `samples` with up to `max` next samples, reusing its memory.
    pub fn read_into(&mut self, samples: &mut Vec<f32>, max: usize) -> io::Result<()> {
        let count = max.min(self.left);
        self.bytes.resize(count * 2, 0);
        self.reader.read_exact(&mut self.bytes)?;
        self.left -= count;
        samples.clear();
        samples.extend(
            self.bytes.chunks_exact(2).map(|pair| {
                f32::from(i16::from_le_bytes([pair[0], pair[1]])) / f32::from(i16::MAX)
            }),
        );
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn stale_name(piece: Piece<'_>) -> String {
    format!(
        "{}.{EXTENSION}",
        stem(piece.theme, piece.mood, piece.index, ALGORITHM + 1)
    )
}
