//! Families of upgrade slots (`upgrade_slots/<name>.ron`): crafted upgrades at the anvil
//! now; machine modules (v0.8), runes and staff parts (v0.9) add their own families
//! without touching the engine.

use crate::id::Identifier;
use crate::pack::PackError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// More would not fit the anvil's screen.
pub const MAX_UPGRADE_SLOTS: usize = 8;

/// A family of upgrade slots: what may go in it is decided by its upgrades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename = "UpgradeSlot", deny_unknown_fields)]
pub struct UpgradeSlotFamily {
    /// At most this many slots of the family on one item.
    #[serde(default = "most")]
    pub max_per_item: u8,
}

fn most() -> u8 {
    MAX_UPGRADE_SLOTS as u8
}

/// Keeps the slots of known families, within the limits; reports the others.
pub(super) fn valid_slots(
    slots: Vec<Identifier>,
    families: &BTreeMap<Identifier, UpgradeSlotFamily>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Vec<Identifier> {
    let mut kept: Vec<Identifier> = Vec::new();
    for slot in slots {
        let Some(family) = families.get(&slot) else {
            errors.push(PackError::invalid(
                source,
                format!("unknown upgrade slot `{slot}`"),
            ));
            continue;
        };
        let same = kept.iter().filter(|kept| **kept == slot).count();
        if same >= usize::from(family.max_per_item) || kept.len() >= MAX_UPGRADE_SLOTS {
            errors.push(PackError::invalid(
                source,
                format!("too many `{slot}` upgrade slots"),
            ));
            continue;
        }
        kept.push(slot);
    }
    kept
}

/// Slots by family, in first-seen order: `[(craft, 2)]`.
pub fn count_slots(slots: &[Identifier]) -> Vec<(&Identifier, usize)> {
    let mut counts: Vec<(&Identifier, usize)> = Vec::new();
    for slot in slots {
        match counts.iter_mut().find(|(family, _)| *family == slot) {
            Some((_, count)) => *count += 1,
            None => counts.push((slot, 1)),
        }
    }
    counts
}
