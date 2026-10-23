use super::{PackError, PackSet, category_files, read_ron};
use crate::id::Identifier;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// An entry and the file it came from (for error messages and the wiki).
#[derive(Debug, Clone)]
pub struct Sourced<T> {
    pub value: T,
    pub source: PathBuf,
}

/// `remove.ron`: entries of lower-priority packs to delete, by category.
/// `{ "blocks": ["strata:sand"], "textures": [] }`
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct Removals(HashMap<String, Vec<Identifier>>);

/// Categories `remove.ron` may name.
pub const REMOVABLE: [&str; 6] = [
    "blocks",
    "items",
    "textures",
    "biomes",
    "features",
    "dimensions",
];

impl Removals {
    pub fn unknown_categories(&self) -> impl Iterator<Item = &str> {
        self.0
            .keys()
            .map(String::as_str)
            .filter(|c| !REMOVABLE.contains(c))
    }

    pub fn get(&self, category: &str) -> &[Identifier] {
        self.0.get(category).map_or(&[], Vec::as_slice)
    }
}

impl PackSet {
    /// Reads `category` in every pack, in priority order: later packs replace or remove
    /// earlier entries. Broken files are reported and skipped.
    pub fn layered<T: DeserializeOwned>(
        &self,
        category: &str,
        errors: &mut Vec<PackError>,
    ) -> BTreeMap<Identifier, Sourced<T>> {
        let mut entries = BTreeMap::new();
        for pack in self.packs() {
            for (id, path) in category_files(pack, category, "ron", errors) {
                match read_ron::<T>(&path) {
                    Ok(value) => {
                        entries.insert(
                            id,
                            Sourced {
                                value,
                                source: path,
                            },
                        );
                    }
                    Err(error) => errors.push(error),
                }
            }
            for id in pack.removals.get(category) {
                if entries.remove(id).is_none() {
                    let message = format!("cannot remove `{id}`: no such entry in `{category}`");
                    errors.push(PackError::invalid(&pack.root.join("remove.ron"), message));
                }
            }
        }
        entries
    }
}
