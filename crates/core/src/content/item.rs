use super::registry::{BlockId, EntityTypeId, TextureId};
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
    /// Uses before it breaks (tools, armour); such an item never stacks.
    #[serde(default)]
    pub durability: Option<u16>,
    /// Equipment slot it may be worn in (armour).
    #[serde(default)]
    pub equip: Option<EquipSlot>,
}

/// Where a worn item goes, in the order of the armour slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, serde::Serialize)]
pub enum EquipSlot {
    Head,
    Chest,
    Legs,
    Feet,
}

impl EquipSlot {
    pub const ALL: [Self; 4] = [Self::Head, Self::Chest, Self::Legs, Self::Feet];
}

/// `action` of an item, naming blocks by identifier.
#[derive(Debug, Clone, Deserialize)]
pub enum ItemActionDef {
    /// Pours a source of this liquid (a full bucket).
    PlaceFluid(Identifier),
    /// Takes the liquid source it is used on (an empty bucket).
    TakeFluid,
    /// Puts this entity on the aimed liquid or block (a boat).
    PlaceEntity(Identifier),
    /// Lights this fire against the aimed face (flint and steel).
    Ignite(Identifier),
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
    pub durability: Option<u16>,
    pub equip: Option<EquipSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    /// Source block of the liquid poured.
    PlaceFluid(BlockId),
    TakeFluid,
    PlaceEntity(EntityTypeId),
    /// Fire block lit.
    Ignite(BlockId),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemKind {
    /// Places this block; drawn as the block.
    Block(BlockId),
    /// Drawn flat with this texture.
    Flat(TextureId),
}
