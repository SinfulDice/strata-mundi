//! Blocks a tool changes in place when used on them (`worked` of a block): tilled by a
//! hoe, carved by a chisel through a cycle of patterns.

use super::{Block, BlockId, Item, ItemId, Registry};
use crate::id::Identifier;
use crate::pack::PackError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// `worked` entry of a block file, under the tool kind that does it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "Worked", deny_unknown_fields)]
pub struct WorkedDef {
    pub into: Identifier,
    /// Only with air above (a hoe tills no buried dirt).
    #[serde(default)]
    pub open_above: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Worked {
    pub into: BlockId,
    pub open_above: bool,
}

/// A block's `worked` entries, kept until every block is known.
pub(super) type WorkedRefs = Vec<(Identifier, BTreeMap<Identifier, WorkedDef>, PathBuf)>;

/// Resolves the target blocks; a tool kind no item has is reported (a typo would make
/// the entry unreachable).
pub(super) fn resolve_worked(
    blocks: &mut Registry<BlockId, Block>,
    refs: WorkedRefs,
    items: &Registry<ItemId, Item>,
    errors: &mut Vec<PackError>,
) {
    let has_kind = |kind: &Identifier| {
        items
            .iter()
            .any(|(_, _, item)| item.tool.as_ref().is_some_and(|tool| tool.is(kind)))
    };
    for (id, entries, source) in refs {
        let mut worked = BTreeMap::new();
        for (kind, def) in entries {
            if !has_kind(&kind) {
                errors.push(PackError::invalid(
                    &source,
                    format!("worked: no item is a `{kind}`"),
                ));
                continue;
            }
            match blocks.key(&def.into) {
                Some(into) => {
                    let open_above = def.open_above;
                    worked.insert(kind, Worked { into, open_above });
                }
                None => errors.push(PackError::invalid(
                    &source,
                    format!("worked: unknown block `{}`", def.into),
                )),
            }
        }
        if let Some(block) = blocks.key(&id).and_then(|key| blocks.get_mut(key)) {
            block.worked = worked.into_iter().collect();
        }
    }
}

impl Block {
    /// What using a tool of `kinds` on this block turns it into.
    pub fn worked_by<'a>(&self, kinds: impl IntoIterator<Item = &'a Identifier>) -> Option<Worked> {
        let kinds: Vec<&Identifier> = kinds.into_iter().collect();
        self.worked
            .iter()
            .find(|(kind, _)| kinds.contains(&kind))
            .map(|(_, worked)| *worked)
    }
}
