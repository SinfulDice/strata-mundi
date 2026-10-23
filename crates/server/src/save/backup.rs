use super::{SaveError, WorldFolder};
use std::fs;
use std::path::Path;

/// Copies the whole world to `<backups>/<id>/<stamp>/` and keeps only the `keep` most
/// recent copies. `stamp` sorts in time order (e.g. `2026-10-23_14-05-09`).
pub fn back_up(
    folder: &WorldFolder,
    backups: &Path,
    stamp: &str,
    keep: usize,
) -> Result<(), SaveError> {
    if keep == 0 {
        return Ok(());
    }
    let root = backups.join(&folder.id);
    copy_dir(&folder.dir, &root.join(stamp))?;
    let mut copies: Vec<_> = fs::read_dir(&root)
        .map_err(|e| SaveError::io(&root, e))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    copies.sort();
    for old in copies.iter().take(copies.len().saturating_sub(keep)) {
        fs::remove_dir_all(old).map_err(|e| SaveError::io(old, e))?;
    }
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), SaveError> {
    fs::create_dir_all(to).map_err(|e| SaveError::io(to, e))?;
    for entry in fs::read_dir(from)
        .map_err(|e| SaveError::io(from, e))?
        .flatten()
    {
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if source.is_dir() {
            copy_dir(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|e| SaveError::io(&source, e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_most_recent_copies() {
        let base = std::env::temp_dir().join(format!("strata-backup-{}", std::process::id()));
        let folder = WorldFolder {
            id: "w".into(),
            dir: base.join("saves/w"),
        };
        fs::create_dir_all(folder.dir.join("regions")).unwrap();
        fs::write(folder.dir.join("regions/r.0.0.region"), b"x").unwrap();
        for stamp in ["1", "2", "3", "4"] {
            back_up(&folder, &base.join("backups"), stamp, 3).unwrap();
        }
        let mut kept: Vec<_> = fs::read_dir(base.join("backups/w"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        kept.sort();
        assert_eq!(kept, ["2", "3", "4"]);
        assert!(base.join("backups/w/4/regions/r.0.0.region").is_file());
        fs::remove_dir_all(base).unwrap();
    }
}
