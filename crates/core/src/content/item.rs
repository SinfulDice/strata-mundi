use super::registry::{BlockId, TextureId};
use crate::id::Identifier;
use serde::Deserialize;

/// An item as written in `items/<name>.ron`. Blocks get their item automatically.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Item", deny_unknown_fields)]
pub struct ItemDef {
    pub texture: Identifier,
    #[serde(default = "default_stack")]
    pub max_stack: u16,
    #[serde(default)]
    pub category: Option<Identifier>,
    /// What using it on the world does (right click).
    #[serde(default)]
    pub action: Option<ItemActionDef>,
}

/// `action` of an item, naming blocks by identifier.
#[derive(Debug, Clone, Deserialize)]
pub enum ItemActionDef {
    /// Pours a source of this liquid (a full bucket).
    PlaceFluid(Identifier),
    /// Takes the liquid source it is used on (an empty bucket).
    TakeFluid,
}

fn default_stack() -> u16 {
    64
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub max_stack: u16,
    pub category: Option<Identifier>,
    pub kind: ItemKind,
    pub action: Option<ItemAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    /// Source block of the liquid poured.
    PlaceFluid(BlockId),
    TakeFluid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemKind {
    /// Places this block; drawn as the block.
    Block(BlockId),
    /// Drawn flat with this texture.
    Flat(TextureId),
}
