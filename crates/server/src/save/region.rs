//! Region files: the modified columns of a 32×32 area, each compressed on its own.
//! Unmodified columns are never written: the generator rebuilds them from the seed.

use super::SaveError;
use super::atomic::write_atomic;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::{Block, BlockId, Registry};
use strata_core::coords::ColumnPos;
use strata_core::id::Identifier;

pub const REGION_SIZE: i32 = 32;
const MAGIC: &[u8; 4] = b"SMRG";
const ZSTD_LEVEL: i32 = 3;

/// Text identifiers of the blocks, both ways: files store identifiers, never numbers
/// (numeric ids change when packs change).
#[derive(Debug)]
pub struct BlockNames {
    names: Vec<Identifier>,
    ids: HashMap<Identifier, BlockId>,
}

impl BlockNames {
    pub fn new(blocks: &Registry<BlockId, Block>) -> Self {
        let names: Vec<Identifier> = blocks.iter().map(|(_, id, _)| id.clone()).collect();
        let ids = blocks
            .iter()
            .map(|(key, id, _)| (id.clone(), key))
            .collect();
        Self { names, ids }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RegionFile {
    /// Key: `z * 32 + x` inside the region; value: zstd-compressed `StoredColumn`.
    columns: BTreeMap<u16, Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredColumn {
    /// Block identifiers; the column's ids are indices into this list.
    blocks: Vec<Identifier>,
    column: Column,
}

/// Saved columns, kept compressed in memory and written region by region on `flush`.
/// Without a folder (benchmarks, tests) it only remembers changes until the world closes.
pub struct RegionStore {
    dir: Option<PathBuf>,
    names: Arc<BlockNames>,
    regions: HashMap<(i32, i32), RegionFile>,
    dirty: HashSet<(i32, i32)>,
}

impl RegionStore {
    pub fn on_disk(dir: PathBuf, names: Arc<BlockNames>) -> Self {
        Self {
            dir: Some(dir),
            ..Self::in_memory(names)
        }
    }

    pub fn in_memory(names: Arc<BlockNames>) -> Self {
        Self {
            dir: None,
            names,
            regions: HashMap::new(),
            dirty: HashSet::new(),
        }
    }

    /// The saved version of a column, if any. Damaged data is reported and ignored.
    pub fn load(&mut self, pos: ColumnPos) -> Option<Column> {
        let (region, key) = locate(pos);
        let names = Arc::clone(&self.names);
        let bytes = self.region(region).columns.get(&key)?;
        match decode(bytes, &names).filter(|column| column.pos == pos) {
            Some(column) => Some(column),
            None => {
                tracing::error!(?pos, "saved column is damaged: generated again");
                None
            }
        }
    }

    /// Keeps a modified column, to be written by the next `flush`.
    pub fn store(&mut self, column: &Column) {
        let Some(bytes) = encode(column, &self.names) else {
            tracing::error!(pos = ?column.pos, "column could not be encoded");
            return;
        };
        let (region, key) = locate(column.pos);
        self.region(region).columns.insert(key, bytes);
        self.dirty.insert(region);
    }

    /// Writes every changed region, each atomically.
    pub fn flush(&mut self) -> Result<(), SaveError> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        for region in std::mem::take(&mut self.dirty) {
            if let Some(file) = self.regions.get(&region) {
                let path = region_path(dir, region);
                let mut bytes = MAGIC.to_vec();
                bytes.extend(postcard::to_stdvec(file).map_err(|e| SaveError::format(&path, e))?);
                write_atomic(&path, &bytes).map_err(|e| SaveError::io(&path, e))?;
            }
        }
        Ok(())
    }

    fn region(&mut self, region: (i32, i32)) -> &mut RegionFile {
        let dir = self.dir.as_deref();
        self.regions
            .entry(region)
            .or_insert_with(|| dir.map(|d| read_region(d, region)).unwrap_or_default())
    }
}

fn locate(pos: ColumnPos) -> ((i32, i32), u16) {
    let region = (pos.x.div_euclid(REGION_SIZE), pos.z.div_euclid(REGION_SIZE));
    let local = pos.z.rem_euclid(REGION_SIZE) * REGION_SIZE + pos.x.rem_euclid(REGION_SIZE);
    (region, local as u16)
}

fn region_path(dir: &Path, (x, z): (i32, i32)) -> PathBuf {
    dir.join(format!("r.{x}.{z}.region"))
}

/// A missing file is an empty region; a damaged one is set aside (`.damaged`) so that
/// saving does not overwrite what might still be recovered by hand.
fn read_region(dir: &Path, region: (i32, i32)) -> RegionFile {
    let path = region_path(dir, region);
    let Ok(bytes) = std::fs::read(&path) else {
        return RegionFile::default();
    };
    let parsed = bytes
        .strip_prefix(MAGIC)
        .and_then(|body| postcard::from_bytes(body).ok());
    parsed.unwrap_or_else(|| {
        tracing::error!(path = %path.display(), "damaged region file set aside");
        let _ = std::fs::rename(&path, path.with_extension("damaged"));
        RegionFile::default()
    })
}

fn encode(column: &Column, names: &BlockNames) -> Option<Vec<u8>> {
    let mut used: Vec<BlockId> = column
        .sections()
        .iter()
        .flat_map(|s| s.palette().iter().copied())
        .collect();
    used.sort_unstable();
    used.dedup();
    let local = |id: BlockId| BlockId(used.binary_search(&id).unwrap_or(0) as u16);
    let stored = StoredColumn {
        blocks: used
            .iter()
            .map(|id| names.names.get(usize::from(id.0)).cloned())
            .collect::<Option<_>>()?,
        column: column.remapped(local)?,
    };
    let raw = postcard::to_stdvec(&stored).ok()?;
    zstd::encode_all(raw.as_slice(), ZSTD_LEVEL).ok()
}

/// Blocks that no longer exist (pack removed) become air.
fn decode(bytes: &[u8], names: &BlockNames) -> Option<Column> {
    let raw = zstd::decode_all(bytes).ok()?;
    let stored: StoredColumn = postcard::from_bytes(&raw).ok()?;
    let current: Vec<BlockId> = stored
        .blocks
        .iter()
        .map(|id| names.ids.get(id).copied().unwrap_or(BlockId::AIR))
        .collect();
    stored.column.remapped(|local| {
        current
            .get(usize::from(local.0))
            .copied()
            .unwrap_or(BlockId::AIR)
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn names(ids: &[&str]) -> Arc<BlockNames> {
        let mut blocks = Registry::default();
        for id in ids {
            blocks.insert(id.parse().unwrap(), Block::air());
        }
        Arc::new(BlockNames::new(&blocks))
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("strata-region-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_modified_column_survives_saving_and_loading() {
        let dir = temp("round-trip");
        let pos = ColumnPos { x: -33, z: 70 };
        let mut column = Column::empty(pos);
        column.set(3, 100, 4, BlockId(1));
        column.set(15, 0, 15, BlockId(2));
        let mut store = RegionStore::on_disk(dir.clone(), names(&["a:air", "a:stone", "a:dirt"]));
        store.store(&column);
        store.flush().unwrap();
        let mut reopened =
            RegionStore::on_disk(dir.clone(), names(&["a:air", "a:stone", "a:dirt"]));
        assert_eq!(reopened.load(pos), Some(column));
        assert_eq!(reopened.load(ColumnPos { x: -34, z: 70 }), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn identifiers_survive_pack_changes() {
        let dir = temp("packs");
        let pos = ColumnPos { x: 1, z: 2 };
        let mut column = Column::empty(pos);
        column.set(0, 10, 0, BlockId(1));
        column.set(1, 10, 0, BlockId(2));
        let mut store = RegionStore::on_disk(dir.clone(), names(&["a:air", "a:stone", "a:gone"]));
        store.store(&column);
        store.flush().unwrap();
        let mut reordered =
            RegionStore::on_disk(dir.clone(), names(&["a:air", "b:new", "a:stone"]));
        let loaded = reordered.load(pos).unwrap();
        assert_eq!(loaded.get(0, 10, 0), BlockId(2), "stone found by name");
        assert_eq!(
            loaded.get(1, 10, 0),
            BlockId::AIR,
            "removed block becomes air"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_damaged_region_is_set_aside() {
        let dir = temp("damaged");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(region_path(&dir, (0, 0)), b"SMRG garbage").unwrap();
        let mut store = RegionStore::on_disk(dir.clone(), names(&["a:air"]));
        assert_eq!(store.load(ColumnPos { x: 0, z: 0 }), None);
        assert!(dir.join("r.0.0.damaged").is_file());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
