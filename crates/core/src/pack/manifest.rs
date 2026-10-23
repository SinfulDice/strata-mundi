use super::{PackError, Removals, read_ron};
use crate::id::is_valid_part;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// `pack.ron` at the root of every pack.
#[derive(Debug, Clone, Deserialize)]
pub struct PackManifest {
    /// Namespace of the entries this pack defines, e.g. `strata`.
    pub namespace: String,
    /// Higher priorities load later and win over lower ones. The base game is 0.
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct Pack {
    pub root: PathBuf,
    pub manifest: PackManifest,
    pub removals: Removals,
}

impl Pack {
    pub fn open(root: &Path) -> Result<Self, PackError> {
        let path = root.join("pack.ron");
        if !path.is_file() {
            return Err(PackError::invalid(root, "missing pack.ron"));
        }
        let manifest: PackManifest = read_ron(&path)?;
        if !is_valid_part(&manifest.namespace) {
            return Err(PackError::invalid(
                &path,
                format!("invalid namespace `{}`", manifest.namespace),
            ));
        }
        let removals_path = root.join("remove.ron");
        let removals: Removals = if removals_path.is_file() {
            read_ron(&removals_path)?
        } else {
            Removals::default()
        };
        if let Some(category) = removals.unknown_categories().next() {
            return Err(PackError::invalid(
                &removals_path,
                format!("unknown category `{category}` (expected blocks, items or textures)"),
            ));
        }
        Ok(Self {
            root: root.to_owned(),
            manifest,
            removals,
        })
    }
}
