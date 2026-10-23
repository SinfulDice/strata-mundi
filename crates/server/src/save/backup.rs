use super::{SaveError, WorldFolder};
use std::fs;
use std::path::Path;
use std::time::Duration;

/// No new copy if the last one is more recent than this: opening a world several times in
/// a row (perhaps after it was damaged) must not push the good copies out.
const MIN_INTERVAL: Duration = Duration::from_secs(10 * 60);

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
    if recent_copy_exists(&root) {
        return Ok(());
    }
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

fn recent_copy_exists(root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    entries.flatten().any(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|time| time.elapsed().ok())
            .is_some_and(|age| age < MIN_INTERVAL)
    })
}

/// Links are not followed (no loops) and leftovers of interrupted writes are skipped.
fn copy_dir(from: &Path, to: &Path) -> Result<(), SaveError> {
    fs::create_dir_all(to).map_err(|e| SaveError::io(to, e))?;
    for entry in fs::read_dir(from)
        .map_err(|e| SaveError::io(from, e))?
        .flatten()
    {
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        let leftover = source
            .extension()
            .is_some_and(|e| e == "tmp" || e == "damaged");
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => copy_dir(&source, &target)?,
            Ok(kind) if kind.is_file() && !leftover => {
                fs::copy(&source, &target).map_err(|e| SaveError::io(&source, e))?;
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Makes a copy look an hour old.
    fn age(dir: &Path) {
        let hour_ago = std::time::SystemTime::now() - Duration::from_secs(3600);
        fs::File::open(dir).unwrap().set_modified(hour_ago).unwrap();
    }

    #[test]
    fn keeps_only_the_most_recent_copies() {
        let base = std::env::temp_dir().join(format!("strata-backup-{}", std::process::id()));
        let folder = WorldFolder {
            id: "w".into(),
            dir: base.join("saves/w"),
        };
        fs::create_dir_all(folder.dir.join("regions")).unwrap();
        fs::write(folder.dir.join("regions/r.0.0.region"), b"x").unwrap();
        fs::write(folder.dir.join("regions/r.0.0.region.tmp"), b"half").unwrap();
        let backups = base.join("backups");
        for old in ["0", "1", "2", "3"] {
            fs::create_dir_all(backups.join("w").join(old)).unwrap();
            age(&backups.join("w").join(old));
        }
        back_up(&folder, &backups, "4", 3).unwrap();
        back_up(&folder, &backups, "5", 3).unwrap();
        let mut kept: Vec<_> = fs::read_dir(backups.join("w"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        kept.sort();
        assert_eq!(kept, ["2", "3", "4"], "one copy, the oldest dropped");
        assert!(backups.join("w/4/regions/r.0.0.region").is_file());
        assert!(!backups.join("w/4/regions/r.0.0.region.tmp").exists());
        fs::remove_dir_all(base).unwrap();
    }
}
