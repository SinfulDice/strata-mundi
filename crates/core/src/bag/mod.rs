//! Bags: items that carry items. A backpack has the slots and upgrade slots of the block
//! it becomes when put down (a container of the `strata:backpack` family, raised by kits
//! like chests); a pouch has a few slots for the items of one tag, filled on pickup. What
//! a bag holds lives in its item's data, so it goes wherever the item goes: no bag is
//! ever emptied by moving, throwing, putting down or breaking it. No bag goes in a bag.

mod contents;
mod describe;
mod pickup;
mod tank;

pub use contents::{BagContents, Tank};
pub use pickup::{BagWorld, bag_slots, pick_into_bags};
pub use tank::{TankUse, use_tank};

use crate::content::{Block, BlockId, ContainerDef, Item, ItemId, ItemTags, Registry};
use crate::id::Identifier;
use crate::inventory::ItemStack;
use crate::pack::PackError;
use crate::storage::{SlotLayout, Storage};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// `bag` of an item file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "Bag", deny_unknown_fields)]
pub struct BagDef {
    /// The block it becomes when put down (sneak + right click): its slots and upgrade
    /// slots are the bag's.
    #[serde(default)]
    pub placed: Option<Identifier>,
    /// Slots of a bag that is never put down (a pouch), 1 to 27.
    #[serde(default)]
    pub slots: u8,
    /// Only the items of this tag go in (a pouch), and they go in first when picked up.
    #[serde(default)]
    pub accepts: Option<Identifier>,
}

impl BagDef {
    pub const MAX_SLOTS: u8 = 27;
}

/// A bag as the engine uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BagRules {
    pub layout: SlotLayout,
    /// The only items it takes (a pouch); any but bags without.
    pub accepts: Option<Vec<ItemId>>,
    pub placed: Option<BlockId>,
}

/// Every bag of the packs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bags {
    items: BTreeMap<ItemId, BagRules>,
    /// The bag item of each placed-bag block.
    blocks: BTreeMap<BlockId, ItemId>,
}

impl Bags {
    pub fn get(&self, item: ItemId) -> Option<&BagRules> {
        self.items.get(&item)
    }

    pub fn is_bag(&self, item: ItemId) -> bool {
        self.items.contains_key(&item)
    }

    /// The bag item a placed bag block is (and gives back when broken).
    pub fn of_block(&self, block: BlockId) -> Option<ItemId> {
        self.blocks.get(&block).copied()
    }

    /// Whether `stack` may lie in `bag`: never a bag, and only its tag's items if it
    /// has one.
    pub fn admits(&self, bag: &BagRules, stack: &ItemStack) -> bool {
        !self.is_bag(stack.item) && bag.accepts.as_ref().is_none_or(|a| a.contains(&stack.item))
    }
}

pub(crate) fn load_bags(
    items: &Registry<ItemId, Item>,
    blocks: &Registry<BlockId, Block>,
    tags: &ItemTags,
    storage: &Storage,
    errors: &mut Vec<PackError>,
) -> Bags {
    let mut bags = Bags::default();
    for (key, id, item) in items.iter() {
        let Some(def) = &item.bag else {
            continue;
        };
        let source = Path::new("items").join(format!("{}/{}.ron", id.namespace(), id.name()));
        let Some(rules) = resolve(def, blocks, tags, storage, &source, errors) else {
            continue;
        };
        if let Some(placed) = rules.placed
            && let Some(other) = bags.blocks.insert(placed, key)
        {
            let name = items.identifier(other).map(ToString::to_string);
            let message = format!(
                "its placed block is already `{}`'s",
                name.unwrap_or_default()
            );
            errors.push(PackError::invalid(&source, message));
        }
        bags.items.insert(key, rules);
    }
    bags
}

fn resolve(
    def: &BagDef,
    blocks: &Registry<BlockId, Block>,
    tags: &ItemTags,
    storage: &Storage,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<BagRules> {
    let accepts = def.accepts.as_ref().map(|tag| {
        let found = tags.get(tag);
        if found.is_none() {
            errors.push(PackError::invalid(source, format!("unknown tag `{tag}`")));
        }
        found.unwrap_or_default().to_vec()
    });
    let Some(placed) = &def.placed else {
        if !(1..=BagDef::MAX_SLOTS).contains(&def.slots) {
            errors.push(PackError::invalid(
                source,
                "bag: 1 to 27 slots, or a placed block",
            ));
            return None;
        }
        let layout = SlotLayout {
            storage: usize::from(def.slots),
            upgrades: 0,
            family: None,
        };
        return Some(BagRules {
            layout,
            accepts,
            placed: None,
        });
    };
    let block = blocks.key(placed);
    let container = block.and_then(|b| blocks.get(b)?.container.as_ref());
    let (Some(block), Some(container)) = (block, container) else {
        let message = format!("bag: `{placed}` is not a container block");
        errors.push(PackError::invalid(source, message));
        return None;
    };
    if container.joins {
        let message = format!("bag: `{placed}` joins its neighbours like a chest");
        errors.push(PackError::invalid(source, message));
        return None;
    }
    Some(BagRules {
        layout: layout_of(container, storage),
        accepts,
        placed: Some(block),
    })
}

fn layout_of(container: &ContainerDef, storage: &Storage) -> SlotLayout {
    let tier = (container.upgradable.as_ref()).and_then(|u| storage.tiers.get(&u.tier));
    SlotLayout {
        storage: container.slots(),
        upgrades: tier.map_or(0, |t| usize::from(t.upgrade_slots)),
        family: tier.map(|t| t.upgrade_family.clone()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
