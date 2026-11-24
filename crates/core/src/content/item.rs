use super::registry::{BlockId, EntityTypeId, ItemId, Registry, TextureId};
use super::{Block, EntityType, FluidLevel, TextureLibrary, ToolDef};
use crate::cooking::KitDef;
use crate::gear::{ArmorDef, Gear, GearFields, ItemGear, ToolSpec, WeaponDef};
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
    pub tool: Option<ToolSpec>,
    /// Material tier (`tiers/`): fills the tool's level and speed, the weapon's damage,
    /// the durability and the upgrade slots the item leaves out.
    #[serde(default)]
    pub tier: Option<Identifier>,
    #[serde(default)]
    pub weapon: Option<WeaponDef>,
    /// Protection and weight of a worn piece (with `equip`).
    #[serde(default)]
    pub armor: Option<ArmorDef>,
    /// Its own upgrade slot families, instead of its tier's (leather, named uniques).
    #[serde(default)]
    pub upgrade_slots: Option<Vec<Identifier>>,
    /// Mends it at the anvil, instead of its tier's material (leather for leather).
    #[serde(default)]
    pub repair: Option<crate::crafting::IngredientDef>,
    /// Left in the crafting grid when used in a recipe (a water bucket leaves a bucket).
    #[serde(default)]
    pub remainder: Option<Identifier>,
    /// Used on a block of a tier, makes it the next tier in place (furnace kits).
    #[serde(default)]
    pub kit: Option<KitDef>,
    /// The upgrade (`upgrades/`) it is once put in a block's upgrade slot (furnace
    /// modules, storage upgrades); such an item never stacks.
    #[serde(default)]
    pub upgrade: Option<Identifier>,
    /// What it gives while worn (with `equip`), in the bricks of upgrades.
    #[serde(default)]
    pub worn: Vec<crate::upgrade::Effect>,
    /// Makes it a bag that carries items (a backpack, a pouch); such an item never stacks.
    #[serde(default)]
    pub bag: Option<crate::bag::BagDef>,
}

/// Where a worn item goes: an armour slot, an accessory slot (two rings, an amulet, a
/// belt, a cloak) or the back (a backpack). Special boots go in the feet slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, serde::Serialize)]
pub enum EquipSlot {
    Head,
    Chest,
    Legs,
    Feet,
    Ring,
    Amulet,
    Belt,
    Cloak,
    Back,
}

impl EquipSlot {
    /// In the order of the armour slots.
    pub const ARMOR: [Self; 4] = [Self::Head, Self::Chest, Self::Legs, Self::Feet];
    /// In the order of the accessory slots.
    pub const ACCESSORIES: [Self; 5] = [
        Self::Ring,
        Self::Ring,
        Self::Amulet,
        Self::Belt,
        Self::Cloak,
    ];
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
    /// Turns the aimed block into what its `worked` says for one of the item's tool
    /// kinds (a hoe tills, a chisel carves).
    Work,
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
    /// What stays in the crafting grid after a recipe uses it.
    pub remainder: Option<ItemId>,
    pub kit: Option<KitDef>,
    /// The upgrade it is in a block's upgrade slot.
    pub upgrade: Option<Identifier>,
    /// As written; resolved in `Content::bags`.
    pub bag: Option<crate::bag::BagDef>,
    /// Tier, weapon and armour stats.
    pub gear: ItemGear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAction {
    /// Source block of the liquid poured.
    PlaceFluid(BlockId),
    TakeFluid,
    PlaceEntity(EntityTypeId),
    /// Fire block lit.
    Ignite(BlockId),
    Work,
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
    pub(super) gear: &'a Gear,
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
                remainder: None,
                kit: None,
                upgrade: None,
                bag: None,
                gear: ItemGear::default(),
            },
        );
    }
    let mut remainders = Vec::new();
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
        let resolved = known.gear.resolve(
            GearFields {
                tier: value.tier,
                tool: value.tool,
                weapon: value.weapon,
                armor: value.armor,
                durability: value.durability,
                equip: value.equip,
                upgrade_slots: value.upgrade_slots,
                repair: value.repair,
                worn: value.worn,
            },
            &source,
            errors,
        );
        let durable = resolved.durability;
        let single = durable.is_some() || value.upgrade.is_some() || value.bag.is_some();
        let item = Item {
            max_stack: if single { 1 } else { value.max_stack.max(1) },
            category: value.category,
            kind: ItemKind::Flat(texture),
            action,
            durability: durable,
            equip: value.equip,
            tool: resolved.tool,
            remainder: None,
            kit: value.kit,
            upgrade: value.upgrade,
            bag: value.bag,
            gear: resolved.gear,
        };
        match items.insert(id.clone(), item) {
            Some(key) => remainders.extend(value.remainder.map(|left| (key, left, source))),
            None => errors.push(PackError::invalid(
                &source,
                format!("`{id}` is already a block item"),
            )),
        }
    }
    for (key, left, source) in remainders {
        let found = items.key(&left);
        if found.is_none() {
            errors.push(PackError::invalid(
                &source,
                format!("remainder: unknown item `{left}`"),
            ));
        }
        if let Some(item) = items.get_mut(key) {
            item.remainder = found;
        }
    }
    items
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
        ItemActionDef::Work => Some(ItemAction::Work),
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
