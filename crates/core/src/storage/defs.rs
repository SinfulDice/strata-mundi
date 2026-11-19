//! What the packs write about storage tiers and compression.

use crate::id::Identifier;
use serde::Deserialize;

/// `storage_tiers/<name>.ron`: the rows of nine slots and the upgrade slots of a chest or
/// barrel of this tier.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "StorageTier", deny_unknown_fields)]
pub struct StorageTier {
    pub rows: u8,
    #[serde(default)]
    pub upgrade_slots: u8,
    /// Family of its upgrade slots (`upgrade_slots/`).
    pub upgrade_family: Identifier,
}

impl StorageTier {
    /// More would be a pack typo: sixteen rows already scroll on every screen.
    pub const MAX_ROWS: u8 = 16;
    pub const MAX_UPGRADE_SLOTS: u8 = 8;
}

/// `upgradable` of a container block: blocks of one family share kits (a kit turns a
/// block into the block of the same family with the next tier), and the tier gives its
/// rows and upgrade slots.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "Upgradable", deny_unknown_fields)]
pub struct Upgradable {
    pub family: Identifier,
    pub tier: Identifier,
}

/// `compression/<name>.ron`: `count` of `from` pressed into one `to`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Compression", deny_unknown_fields)]
pub struct CompressionDef {
    pub from: Identifier,
    pub to: Identifier,
    #[serde(default = "nine")]
    pub count: u16,
}

fn nine() -> u16 {
    9
}
