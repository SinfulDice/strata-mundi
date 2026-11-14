//! Item tags (`tags/<name>.ron`): named groups of items such as « any planks », used by
//! recipes. Unlike other entries, a tag of a later pack adds to the earlier one unless it
//! says `replace: true`, so a pack adding a wood only lists its own planks.

use super::Item;
use super::registry::{ItemId, Registry};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, category_files, read_ron};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Tag", deny_unknown_fields)]
pub struct TagDef {
    pub items: Vec<Identifier>,
    /// Forget what lower-priority packs put in this tag.
    #[serde(default)]
    pub replace: bool,
}

/// Every item tag, each a sorted list of items.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemTags(BTreeMap<Identifier, Vec<ItemId>>);

impl ItemTags {
    pub fn get(&self, tag: &Identifier) -> Option<&[ItemId]> {
        self.0.get(tag).map(Vec::as_slice)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &[ItemId])> {
        self.0.iter().map(|(id, items)| (id, items.as_slice()))
    }

    /// Sets `tag` to `items` (sorted, without repeats), replacing what it held.
    pub fn insert(&mut self, tag: Identifier, mut items: Vec<ItemId>) {
        items.sort();
        items.dedup();
        self.0.insert(tag, items);
    }
}

pub(super) fn load_tags(
    packs: &PackSet,
    items: &Registry<ItemId, Item>,
    errors: &mut Vec<PackError>,
) -> ItemTags {
    let mut tags: BTreeMap<Identifier, Vec<ItemId>> = BTreeMap::new();
    for pack in packs.packs() {
        for (id, path) in category_files(pack, "tags", "ron", errors) {
            let def = match read_ron::<TagDef>(&path) {
                Ok(def) => def,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            let entry = tags.entry(id).or_default();
            if def.replace {
                entry.clear();
            }
            for item in def.items {
                match items.key(&item) {
                    Some(key) => entry.push(key),
                    None => {
                        errors.push(PackError::invalid(&path, format!("unknown item `{item}`")))
                    }
                }
            }
        }
        for id in pack.removals.get("tags") {
            if tags.remove(id).is_none() {
                let message = format!("cannot remove `{id}`: no such entry in `tags`");
                errors.push(PackError::invalid(&pack.root.join("remove.ron"), message));
            }
        }
    }
    let mut resolved = ItemTags::default();
    for (id, list) in tags {
        resolved.insert(id, list);
    }
    resolved
}
