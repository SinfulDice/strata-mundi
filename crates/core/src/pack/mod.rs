//! Content packs: `assets/packs/<namespace>/` folders of RON files, loaded by priority.
//! A pack can add entries, replace another pack's entries, or remove them.

mod error;
mod layered;
mod manifest;

pub use error::PackError;
pub use layered::{Removals, Sourced};
pub use manifest::{Pack, PackManifest};

use crate::id::{Identifier, is_valid_part};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

/// Every pack found in a folder, in load order (lowest priority first).
#[derive(Debug, Default)]
pub struct PackSet {
    packs: Vec<Pack>,
}

impl PackSet {
    /// Packs with an invalid manifest are skipped and reported in `errors`.
    pub fn discover(packs_dir: &Path, errors: &mut Vec<PackError>) -> Self {
        let mut packs: Vec<Pack> = subdirectories(packs_dir, errors)
            .into_iter()
            .filter_map(|dir| Pack::open(&dir).map_err(|e| errors.push(e)).ok())
            .collect();
        packs.sort_by(|a, b| {
            (a.manifest.priority, &a.manifest.namespace)
                .cmp(&(b.manifest.priority, &b.manifest.namespace))
        });
        for pair in packs.windows(2) {
            if pair[0].manifest.namespace == pair[1].manifest.namespace {
                let message = format!(
                    "namespace `{}` is used by two packs",
                    pair[1].manifest.namespace
                );
                errors.push(PackError::invalid(&pair[1].root, message));
            }
        }
        Self { packs }
    }

    /// The packs `names` inside `dir`, in that order (last wins): the resource packs the
    /// player activated. Missing or broken ones are reported and skipped.
    pub fn named(dir: &Path, names: &[String], errors: &mut Vec<PackError>) -> Self {
        let packs = names
            .iter()
            .filter_map(|name| Pack::open(&dir.join(name)).map_err(|e| errors.push(e)).ok())
            .collect();
        Self { packs }
    }

    /// `base` then `top`, `top` winning.
    pub fn joined(base: &PackSet, top: &PackSet) -> Self {
        Self {
            packs: base.packs.iter().chain(&top.packs).cloned().collect(),
        }
    }

    pub fn packs(&self) -> &[Pack] {
        &self.packs
    }
}

/// Files of `category` in `pack` with the given extension, with the identifier each defines:
/// `blocks/stone.ron` in pack `strata` → `strata:stone`; `blocks/strata/stone.ron` in any
/// pack → `strata:stone` too (an override of another pack's entry).
pub fn category_files(
    pack: &Pack,
    category: &str,
    extension: &str,
    errors: &mut Vec<PackError>,
) -> Vec<(Identifier, PathBuf)> {
    let dir = pack.root.join(category);
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut found = files_with_extension(&dir, &pack.manifest.namespace, extension, errors);
    for namespace_dir in subdirectories(&dir, errors) {
        let namespace = file_stem(&namespace_dir);
        if !is_valid_part(&namespace) {
            errors.push(PackError::invalid(
                &namespace_dir,
                "folder name is not a valid namespace",
            ));
            continue;
        }
        found.extend(files_with_extension(
            &namespace_dir,
            &namespace,
            extension,
            errors,
        ));
    }
    found.sort();
    found
}

pub fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, PackError> {
    let text = std::fs::read_to_string(path).map_err(|e| PackError::io(path, e))?;
    ron::from_str(&text).map_err(|e| PackError::syntax(path, &e))
}

fn files_with_extension(
    dir: &Path,
    namespace: &str,
    extension: &str,
    errors: &mut Vec<PackError>,
) -> Vec<(Identifier, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir).map_err(|e| errors.push(PackError::io(dir, e))) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == extension))
        .filter_map(|path| match Identifier::new(namespace, &file_stem(&path)) {
            Ok(id) => Some((id, path)),
            Err(error) => {
                errors.push(PackError::invalid(&path, error.to_string()));
                None
            }
        })
        .collect()
}

fn subdirectories(dir: &Path, errors: &mut Vec<PackError>) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir).map_err(|e| errors.push(PackError::io(dir, e))) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}
