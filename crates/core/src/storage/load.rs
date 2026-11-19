//! Reading `storage_tiers/` and `compression/`; container blocks of a tier take its rows.
//! Every unknown name is reported with its file.

use super::defs::{CompressionDef, StorageTier};
use super::{Compression, Storage};
use crate::content::{Block, BlockId, Item, ItemId, Registry};
use crate::cooking::{Cooking, KitTarget};
use crate::gear::UpgradeSlotFamily;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) fn load_storage(
    packs: &PackSet,
    items: &Registry<ItemId, Item>,
    blocks: &mut Registry<BlockId, Block>,
    (cooking, families): (&Cooking, &BTreeMap<Identifier, UpgradeSlotFamily>),
    errors: &mut Vec<PackError>,
) -> Storage {
    let tiers: BTreeMap<Identifier, StorageTier> = packs
        .layered::<StorageTier>("storage_tiers", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            valid_tier(&value, families, &source, errors).then_some((id, value))
        })
        .collect();
    for (id, tier) in cooking.tiers() {
        if !families.contains_key(&tier.upgrade_family) {
            let message = format!("unknown upgrade slot `{}`", tier.upgrade_family);
            errors.push(PackError::invalid(&place("furnace_tiers", id), message));
        }
    }
    resolve_containers(blocks, &tiers, errors);
    check_kits(items, &tiers, errors);
    let compressions = packs
        .layered::<CompressionDef>("compression", errors)
        .into_iter()
        .filter_map(|(_, Sourced { value, source })| compression(value, items, &source, errors))
        .collect();
    Storage {
        tiers,
        compressions,
    }
}

fn place(folder: &str, id: &Identifier) -> PathBuf {
    Path::new(folder).join(format!("{}/{}.ron", id.namespace(), id.name()))
}

fn valid_tier(
    tier: &StorageTier,
    families: &BTreeMap<Identifier, UpgradeSlotFamily>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> bool {
    let mut fail = |message: String| errors.push(PackError::invalid(source, message));
    if !(1..=StorageTier::MAX_ROWS).contains(&tier.rows) {
        fail(format!("1 to {} rows", StorageTier::MAX_ROWS));
        return false;
    }
    if tier.upgrade_slots > StorageTier::MAX_UPGRADE_SLOTS {
        fail(format!(
            "at most {} upgrade slots",
            StorageTier::MAX_UPGRADE_SLOTS
        ));
        return false;
    }
    if !families.contains_key(&tier.upgrade_family) {
        fail(format!("unknown upgrade slot `{}`", tier.upgrade_family));
        return false;
    }
    true
}

/// A container of a tier has the tier's rows; one of an unknown tier is no container.
fn resolve_containers(
    blocks: &mut Registry<BlockId, Block>,
    tiers: &BTreeMap<Identifier, StorageTier>,
    errors: &mut Vec<PackError>,
) {
    for (_, id, block) in blocks.iter_mut() {
        let Some(container) = &mut block.container else {
            continue;
        };
        let Some(upgradable) = &container.upgradable else {
            continue;
        };
        match tiers.get(&upgradable.tier) {
            Some(tier) => container.rows = tier.rows,
            None => {
                let message = format!("unknown storage tier `{}`", upgradable.tier);
                errors.push(PackError::invalid(&place("blocks", id), message));
                block.container = None;
            }
        }
    }
}

fn check_kits(
    items: &Registry<ItemId, Item>,
    tiers: &BTreeMap<Identifier, StorageTier>,
    errors: &mut Vec<PackError>,
) {
    for (_, id, item) in items.iter() {
        let Some(kit) = item.kit.as_ref().filter(|kit| kit.on == KitTarget::Storage) else {
            continue;
        };
        for tier in [&kit.from, &kit.to]
            .into_iter()
            .filter(|t| !tiers.contains_key(t))
        {
            let message = format!("kit: unknown storage tier `{tier}`");
            errors.push(PackError::invalid(&place("items", id), message));
        }
    }
}

fn compression(
    def: CompressionDef,
    items: &Registry<ItemId, Item>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<Compression> {
    let mut find = |id: &Identifier| {
        let found = items.key(id);
        if found.is_none() {
            errors.push(PackError::invalid(source, format!("unknown item `{id}`")));
        }
        found
    };
    let (from, to) = (find(&def.from), find(&def.to));
    Some(Compression {
        from: from?,
        to: to?,
        count: def.count.clamp(2, 64),
    })
}
