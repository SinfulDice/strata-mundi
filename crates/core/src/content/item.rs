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
}

fn default_stack() -> u16 {
    64
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub max_stack: u16,
    pub category: Option<Identifier>,
    pub kind: ItemKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemKind {
    /// Places this block; drawn as the block.
    Block(BlockId),
    /// Drawn flat with this texture.
    Flat(TextureId),
}
