//! File helpers shared by saves and settings.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Writes under a temporary name, flushes to the disk, then renames over the target: a
/// power cut (DS battery) leaves either the old file or the new one, never half of one.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = Path::new(&temporary);
    {
        let mut file = File::create(temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(temporary, path)?;
    // Makes the rename itself durable; not possible (nor needed) on every system.
    if let Some(dir) = path.parent() {
        let _ = File::open(dir).and_then(|dir| dir.sync_all());
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_file_and_leaves_no_temporary() {
        let dir = std::env::temp_dir().join(format!("strata-atomic-{}", std::process::id()));
        let path = dir.join("a/b.ron");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        assert_eq!(fs::read_dir(dir.join("a")).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
