//! Worlds on disk, one folder each:
//!
//! ```text
//! <saves>/<id>/world.ron                          description, versioned
//!              player.ron                         the player's character
//!              icon.png                           capture taken at the last save
//!              regions/<namespace>/<dimension>/   modified columns only
//! <backups>/<id>/<date>/                          rotating copies
//! ```

mod atomic;
mod backup;
mod meta;
mod region;

pub use atomic::write_atomic;
pub use backup::back_up;
pub use meta::{FORMAT_VERSION, PlayerData, WorldMeta};
pub use region::{BlockNames, REGION_SIZE, RegionStore};

use meta::FormatOnly;
use std::path::{Path, PathBuf};
use strata_core::dimension::DimensionId;
use strata_protocol::{NewWorld, WorldStatus, WorldSummary};
use thiserror::Error;

const META_FILE: &str = "world.ron";
const PLAYER_FILE: &str = "player.ron";
const ICON_FILE: &str = "icon.png";
const LONGEST_ID: usize = 48;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Format { path: PathBuf, message: String },
    #[error("unknown world `{0}`")]
    UnknownWorld(String),
    #[error("world saved in format {found}, this version reads format {FORMAT_VERSION}")]
    Incompatible { found: u32 },
}

impl SaveError {
    fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.to_owned(),
            source,
        }
    }

    fn format(path: &Path, message: impl ToString) -> Self {
        Self::Format {
            path: path.to_owned(),
            message: message.to_string(),
        }
    }
}

/// The folder of one world.
#[derive(Debug, Clone)]
pub struct WorldFolder {
    pub id: String,
    pub dir: PathBuf,
}

impl WorldFolder {
    pub fn regions(&self, dimension: &DimensionId) -> PathBuf {
        let id = &dimension.0;
        self.dir
            .join("regions")
            .join(id.namespace())
            .join(id.name())
    }

    pub fn read_meta(&self) -> Result<WorldMeta, SaveError> {
        let path = self.dir.join(META_FILE);
        let text = std::fs::read_to_string(&path).map_err(|e| SaveError::io(&path, e))?;
        let format = ron::from_str::<FormatOnly>(&text)
            .map_err(|e| SaveError::format(&path, e))?
            .format;
        if format != FORMAT_VERSION {
            return Err(SaveError::Incompatible { found: format });
        }
        ron::from_str(&text).map_err(|e| SaveError::format(&path, e))
    }

    pub fn write_meta(&self, meta: &WorldMeta) -> Result<(), SaveError> {
        self.write_ron(META_FILE, meta)
    }

    /// `None` for a world never played.
    pub fn read_player(&self) -> Result<Option<PlayerData>, SaveError> {
        let path = self.dir.join(PLAYER_FILE);
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path).map_err(|e| SaveError::io(&path, e))?;
        ron::from_str(&text)
            .map(Some)
            .map_err(|e| SaveError::format(&path, e))
    }

    pub fn write_player(&self, player: &PlayerData) -> Result<(), SaveError> {
        self.write_ron(PLAYER_FILE, player)
    }

    pub fn write_icon(&self, png: &[u8]) -> Result<(), SaveError> {
        let path = self.dir.join(ICON_FILE);
        write_atomic(&path, png).map_err(|e| SaveError::io(&path, e))
    }

    fn write_ron(&self, name: &str, value: &impl serde::Serialize) -> Result<(), SaveError> {
        let path = self.dir.join(name);
        let text = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
            .map_err(|e| SaveError::format(&path, e))?;
        write_atomic(&path, text.as_bytes()).map_err(|e| SaveError::io(&path, e))
    }
}

/// Every world of `saves`, most recently played first.
pub fn list(saves: &Path) -> Vec<WorldSummary> {
    let Ok(entries) = std::fs::read_dir(saves) else {
        return Vec::new();
    };
    let mut worlds: Vec<WorldSummary> = entries
        .flatten()
        .filter(|entry| entry.path().join(META_FILE).is_file())
        .filter_map(|entry| entry.file_name().to_str().map(|id| summary(saves, id)))
        .collect();
    worlds.sort_by(|a, b| b.last_played.cmp(&a.last_played).then(a.name.cmp(&b.name)));
    worlds
}

fn summary(saves: &Path, id: &str) -> WorldSummary {
    let folder = WorldFolder {
        id: id.to_owned(),
        dir: saves.join(id),
    };
    let icon = std::fs::read(folder.dir.join(ICON_FILE)).ok();
    let (meta, status) = match folder.read_meta() {
        Ok(meta) => (Some(meta), WorldStatus::Ready),
        Err(SaveError::Incompatible { found }) if found < FORMAT_VERSION => {
            (None, WorldStatus::TooOld)
        }
        Err(SaveError::Incompatible { .. }) => (None, WorldStatus::TooNew),
        Err(error) => {
            tracing::warn!(%error, "unreadable world");
            (None, WorldStatus::Broken)
        }
    };
    WorldSummary {
        id: id.to_owned(),
        name: meta
            .as_ref()
            .map_or_else(|| id.to_owned(), |m| m.name.clone()),
        mode: meta.as_ref().map(|m| m.mode).unwrap_or_default(),
        last_played: meta.as_ref().map_or(0, |m| m.last_played),
        play_seconds: meta.as_ref().map_or(0, |m| m.play_ticks / 20),
        icon,
        status,
    }
}

/// Creates the folder (named after the world, made unique) and its description.
pub fn create(
    saves: &Path,
    new: &NewWorld,
    seed: u64,
    now: i64,
) -> Result<(WorldFolder, WorldMeta), SaveError> {
    let base = folder_name(&new.name);
    let id = std::iter::once(base.clone())
        .chain((2..1000).map(|n| format!("{base}-{n}")))
        .find(|id| !saves.join(id).exists())
        .ok_or_else(|| SaveError::UnknownWorld(base.clone()))?;
    let folder = WorldFolder {
        dir: saves.join(&id),
        id,
    };
    std::fs::create_dir_all(&folder.dir).map_err(|e| SaveError::io(&folder.dir, e))?;
    let meta = WorldMeta {
        format: FORMAT_VERSION,
        game_version: env!("CARGO_PKG_VERSION").to_owned(),
        name: new.name.trim().to_owned(),
        seed,
        seed_text: new.seed.trim().to_owned(),
        mode: new.mode,
        border: new.border,
        created: now,
        last_played: now,
        play_ticks: 0,
    };
    folder.write_meta(&meta)?;
    Ok((folder, meta))
}

/// An existing world, by the id the world list gave.
pub fn open(saves: &Path, id: &str) -> Result<(WorldFolder, WorldMeta), SaveError> {
    if !is_folder_name(id) || !saves.join(id).is_dir() {
        return Err(SaveError::UnknownWorld(id.to_owned()));
    }
    let folder = WorldFolder {
        id: id.to_owned(),
        dir: saves.join(id),
    };
    let meta = folder.read_meta()?;
    Ok((folder, meta))
}

/// Deletes a world for good (the menu asks for confirmation first; backups are kept).
pub fn delete(saves: &Path, id: &str) -> Result<(), SaveError> {
    if !is_folder_name(id) {
        return Err(SaveError::UnknownWorld(id.to_owned()));
    }
    let dir = saves.join(id);
    std::fs::remove_dir_all(&dir).map_err(|e| SaveError::io(&dir, e))
}

/// Lowercase ASCII letters, digits and dashes: safe on every file system.
fn folder_name(name: &str) -> String {
    let mut id = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        let c = match c {
            'a'..='z' | '0'..='9' => c,
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            _ => '-',
        };
        if !(c == '-' && (id.is_empty() || id.ends_with('-'))) && id.len() < LONGEST_ID {
            id.push(c);
        }
    }
    let id = id.trim_end_matches('-');
    if id.is_empty() {
        "world".to_owned()
    } else {
        id.to_owned()
    }
}

fn is_folder_name(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= LONGEST_ID + 4
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use strata_protocol::GameMode;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("strata-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn new_world(name: &str) -> NewWorld {
        NewWorld {
            name: name.to_owned(),
            seed: "42".to_owned(),
            mode: GameMode::Creative,
            border: 1000,
        }
    }

    #[test]
    fn folder_names_are_safe_and_unique() {
        assert_eq!(folder_name("  Mon Château !"), "mon-chateau");
        assert_eq!(folder_name("../.."), "world");
        assert!(!is_folder_name("../x"));
        let saves = temp("names");
        let (a, _) = create(&saves, &new_world("Île"), 1, 0).unwrap();
        let (b, _) = create(&saves, &new_world("Île"), 1, 0).unwrap();
        assert_eq!((a.id.as_str(), b.id.as_str()), ("ile", "ile-2"));
        std::fs::remove_dir_all(saves).unwrap();
    }

    #[test]
    fn worlds_are_listed_opened_and_deleted() {
        let saves = temp("list");
        let (folder, meta) = create(&saves, &new_world("Alpha"), 7, 100).unwrap();
        let listed = list(&saves);
        assert_eq!(listed.len(), 1);
        assert_eq!(
            (listed[0].name.as_str(), listed[0].status),
            ("Alpha", WorldStatus::Ready)
        );
        assert_eq!(open(&saves, &folder.id).unwrap().1, meta);
        delete(&saves, &folder.id).unwrap();
        assert!(list(&saves).is_empty());
        std::fs::remove_dir_all(saves).unwrap();
    }

    #[test]
    fn other_formats_are_refused_with_a_clear_status() {
        let saves = temp("format");
        let (folder, mut meta) = create(&saves, &new_world("Old"), 7, 100).unwrap();
        meta.format = 0;
        folder.write_meta(&meta).unwrap();
        assert_eq!(list(&saves)[0].status, WorldStatus::TooOld);
        assert!(matches!(
            open(&saves, &folder.id),
            Err(SaveError::Incompatible { found: 0 })
        ));
        std::fs::remove_dir_all(saves).unwrap();
    }
}
