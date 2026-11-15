//! Sets of matching pieces (`sets/<name>.ron`): wearing 2, 3 or 4 of them gives growing
//! bonuses, written with the same effect bricks as upgrades.

use crate::content::ItemId;
use crate::id::Identifier;
use crate::upgrade::Effect;
use serde::Deserialize;

/// A set as written in its file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Set", deny_unknown_fields)]
pub struct SetDef {
    /// The items of the set; each different piece worn counts once.
    pub pieces: Vec<Identifier>,
    /// From the fewest pieces; the largest reached counts alone (it repeats what it keeps
    /// of the smaller ones).
    pub bonuses: Vec<BonusDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Bonus", deny_unknown_fields)]
pub struct BonusDef {
    pub pieces: u8,
    pub effects: Vec<Effect>,
}

/// A set, its pieces resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct GearSet {
    pub id: Identifier,
    pub pieces: Vec<ItemId>,
    /// Fewest pieces first.
    pub bonuses: Vec<SetBonus>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetBonus {
    pub pieces: u8,
    pub effects: Vec<Effect>,
}

impl GearSet {
    /// The bonus `worn` different pieces give, if any.
    pub fn bonus(&self, worn: u8) -> Option<usize> {
        self.bonuses.iter().rposition(|bonus| bonus.pieces <= worn)
    }

    pub fn size(&self) -> u8 {
        u8::try_from(self.pieces.len()).unwrap_or(u8::MAX)
    }
}
