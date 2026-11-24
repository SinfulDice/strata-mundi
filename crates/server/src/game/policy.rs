//! The rules of the open window's slots: a furnace's, a storage's with upgrade slots, a
//! bag's (no bag inside, a pouch's tag).

use super::OpenWindow;
use crate::rules::BlockRules;
use strata_core::cooking::FurnaceSlots;
use strata_core::inventory::SlotPolicy;
use strata_core::storage::{ItemFilter, StorageSlots};
use strata_core::upgrade::SlotOwner;

/// The rules of the open container's slots: a furnace's, or a storage's with upgrade
/// slots; none for plain storage.
pub(super) enum OpenPolicy<'a> {
    Furnace(FurnaceSlots<'a>),
    Storage(StorageSlots<'a>),
}

impl OpenPolicy<'_> {
    pub(super) fn as_dyn(&self) -> &dyn SlotPolicy {
        match self {
            Self::Furnace(policy) => policy,
            Self::Storage(policy) => policy,
        }
    }
}

pub(super) fn policy<'a>(
    rules: &'a BlockRules,
    window: &OpenWindow,
    filters: Vec<ItemFilter>,
) -> Option<OpenPolicy<'a>> {
    let bag = window.bag.and_then(|item| rules.bags.get(item));
    if let Some(bag) = bag {
        return Some(OpenPolicy::Storage(StorageSlots {
            items: rules.items(),
            upgrades: &rules.upgrades,
            owner: SlotOwner {
                family: bag.layout.family.as_ref(),
                item: window.bag,
            },
            storage: bag.layout.storage,
            filters,
            bag: Some((&rules.bags, bag)),
        }));
    }
    let layout = rules.layout(window.block)?;
    let owner = SlotOwner {
        family: Some(layout.family.as_ref()?),
        item: rules.block_item(window.block),
    };
    if let Some(furnace) = rules.furnace(window.block) {
        return Some(OpenPolicy::Furnace(FurnaceSlots {
            cooking: &rules.cooking,
            items: rules.items(),
            upgrades: &rules.upgrades,
            kinds: &furnace.kinds,
            owner,
        }));
    }
    (layout.upgrades > 0).then(|| {
        OpenPolicy::Storage(StorageSlots {
            items: rules.items(),
            upgrades: &rules.upgrades,
            owner,
            storage: layout.storage * window.halves.len(),
            filters,
            bag: None,
        })
    })
}
