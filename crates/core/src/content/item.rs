use super::registry::{BlockId, EntityTypeId, ItemId, Registry, TextureId};
use super::{Block, EntityType, FluidLevel, TextureLibrary, ToolDef};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
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
    /// What it mines and how well (pickaxes, axes, shears…).
    #[serde(default)]
    pub tool: Option<ToolDef>,
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
    pub tool: Option<ToolDef>,
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

/// What items may refer to.
pub(super) struct Known<'a> {
    pub(super) blocks: &'a Registry<BlockId, Block>,
    pub(super) entities: &'a Registry<EntityTypeId, EntityType>,
}

pub(super) fn load_items(
    packs: &PackSet,
    known: &Known<'_>,
    textures: &TextureLibrary,
    errors: &mut Vec<PackError>,
) -> Registry<ItemId, Item> {
    let mut items = Registry::default();
    for (block_id, id, block) in known.blocks.iter().filter(|(_, _, b)| b.has_item) {
        items.insert(
            id.clone(),
            Item {
                max_stack: block.max_stack,
                category: block.category.clone(),
                kind: ItemKind::Block(block_id),
                action: None,
                durability: None,
                equip: None,
                tool: None,
            },
        );
    }
    for (id, Sourced { value, source }) in packs.layered::<ItemDef>("items", errors) {
        let texture = textures.layer(&value.texture).unwrap_or_else(|| {
            errors.push(PackError::invalid(
                &source,
                format!("unknown texture `{}`", value.texture),
            ));
            TextureId::MISSING
        });
        let action = value
            .action
            .and_then(|action| item_action(known, action, &source, errors));
        let durable = value.durability.filter(|&uses| uses > 0);
        let item = Item {
            max_stack: if durable.is_some() {
                1
            } else {
                value.max_stack.max(1)
            },
            category: value.category,
            kind: ItemKind::Flat(texture),
            action,
            durability: durable,
            equip: value.equip,
            tool: value
                .tool
                .and_then(|tool| valid_tool(tool, &source, errors)),
        };
        if items.insert(id.clone(), item).is_none() {
            errors.push(PackError::invalid(
                &source,
                format!("`{id}` is already a block item"),
            ));
        }
    }
    items
}

fn valid_tool(
    tool: ToolDef,
    source: &std::path::Path,
    errors: &mut Vec<PackError>,
) -> Option<ToolDef> {
    if tool.kinds.is_empty() {
        errors.push(PackError::invalid(source, "a tool needs at least one kind"));
        return None;
    }
    Some(tool.clamped())
}

fn item_action(
    known: &Known<'_>,
    action: ItemActionDef,
    source: &std::path::Path,
    errors: &mut Vec<PackError>,
) -> Option<ItemAction> {
    let blocks = known.blocks;
    match action {
        ItemActionDef::TakeFluid => Some(ItemAction::TakeFluid),
        ItemActionDef::PlaceEntity(id) => {
            let entity = known.entities.key(&id);
            if entity.is_none() {
                errors.push(PackError::invalid(
                    source,
                    format!("PlaceEntity: unknown entity `{id}`"),
                ));
            }
            entity.map(ItemAction::PlaceEntity)
        }
        ItemActionDef::PlaceFluid(id) => {
            let fluid = blocks.key(&id).filter(|&key| {
                blocks
                    .get(key)
                    .and_then(|block| block.fluid)
                    .is_some_and(|state| state.level == FluidLevel::Source)
            });
            if fluid.is_none() {
                errors.push(PackError::invalid(
                    source,
                    format!("PlaceFluid: `{id}` is not a liquid source"),
                ));
            }
            fluid.map(ItemAction::PlaceFluid)
        }
        ItemActionDef::Ignite(id) => {
            let fire = blocks
                .key(&id)
                .filter(|&key| blocks.get(key).is_some_and(|block| block.fire.is_some()));
            if fire.is_none() {
                errors.push(PackError::invalid(
                    source,
                    format!("Ignite: `{id}` is not a fire"),
                ));
            }
            fire.map(ItemAction::Ignite)
        }
    }
}
