//! Reading `furnace_tiers/`, `cooking/`, `stonecutting/` and `fuels/`, with every unknown
//! name reported with its file.

use super::defs::{CookingDef, CuttingDef, FuelDef, FurnaceTier};
use super::{Cooking, CookingRecipe, CuttingRecipe};
use crate::content::{Block, BlockId, Item, ItemId, Registry};
use crate::crafting::{Ingredient, Names, ResultDef, ingredient};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Longest a recipe or a fuel may take: a little under a day of play.
const MAX_TICKS: u32 = 1_000_000;

pub(crate) fn load_cooking(
    packs: &PackSet,
    names: &Names<'_>,
    blocks: &Registry<BlockId, Block>,
    errors: &mut Vec<PackError>,
) -> Cooking {
    let tiers: BTreeMap<Identifier, FurnaceTier> = packs
        .layered::<FurnaceTier>("furnace_tiers", errors)
        .into_iter()
        .map(|(id, Sourced { value, .. })| (id, value))
        .collect();
    check_tiers(names.items, blocks, &tiers, errors);
    let recipes = packs
        .layered::<CookingDef>("cooking", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            let ingredient = ingredient(value.ingredient, names, &source, errors)?;
            let (result, count) = result(&value.result, names.items, &source, errors)?;
            Some(CookingRecipe {
                id,
                ingredient,
                result,
                count,
                time: ticks(value.time, &source, errors),
                experience: value.experience.clamp(0.0, 100.0),
                kind: value.kind,
            })
        })
        .collect();
    let cuttings = packs
        .layered::<CuttingDef>("stonecutting", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            let ingredient = ingredient(value.ingredient, names, &source, errors)?;
            let (result, count) = result(&value.result, names.items, &source, errors)?;
            Some(CuttingRecipe {
                id,
                ingredient,
                result,
                count,
            })
        })
        .collect();
    Cooking::new(recipes, cuttings, fuels(packs, names, errors), tiers)
}

/// An item named alone wins over its tag (its own time over the tag's). Also the time of
/// the reference fuel.
fn fuels(
    packs: &PackSet,
    names: &Names<'_>,
    errors: &mut Vec<PackError>,
) -> (HashMap<ItemId, u32>, Option<u32>) {
    let mut fuels = HashMap::new();
    let mut reference = None;
    let mut by_tag: Vec<(bool, Ingredient, u32)> = Vec::new();
    for (_, Sourced { value, source }) in packs.layered::<FuelDef>("fuels", errors) {
        let burn = ticks(value.burn_time, &source, errors);
        if value.reference {
            reference = reference.or(Some(burn));
        }
        if let Some(ingredient) = ingredient(value.ingredient, names, &source, errors) {
            by_tag.push((ingredient.tag.is_none(), ingredient, burn));
        }
    }
    by_tag.sort_by_key(|(single, _, _)| *single);
    for (_, ingredient, burn) in by_tag {
        for &item in ingredient.items() {
            fuels.insert(item, burn);
        }
    }
    (fuels, reference)
}

fn ticks(ticks: u32, source: &Path, errors: &mut Vec<PackError>) -> u32 {
    if !(1..=MAX_TICKS).contains(&ticks) {
        errors.push(PackError::invalid(
            source,
            format!("time: 1 to {MAX_TICKS} ticks"),
        ));
    }
    ticks.clamp(1, MAX_TICKS)
}

fn result(
    def: &ResultDef,
    items: &Registry<ItemId, Item>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<(ItemId, u16)> {
    let Some(item) = items.key(&def.item) else {
        let message = format!("unknown result item `{}`", def.item);
        errors.push(PackError::invalid(source, message));
        return None;
    };
    let max = items.get(item).map_or(1, |entry| entry.max_stack.max(1));
    if !(1..=max).contains(&def.count) {
        errors.push(PackError::invalid(
            source,
            format!("result count: 1 to {max}"),
        ));
    }
    Some((item, def.count.clamp(1, max)))
}

/// Furnace blocks and kits must name tiers that exist.
fn check_tiers(
    items: &Registry<ItemId, Item>,
    blocks: &Registry<BlockId, Block>,
    tiers: &BTreeMap<Identifier, FurnaceTier>,
    errors: &mut Vec<PackError>,
) {
    let place = |folder: &str, id: &Identifier| {
        Path::new(folder).join(format!("{}/{}.ron", id.namespace(), id.name()))
    };
    for (_, id, block) in blocks.iter() {
        let unknown = block
            .furnace
            .as_ref()
            .filter(|f| !tiers.contains_key(&f.tier) && block.has_item);
        if let Some(furnace) = unknown {
            let message = format!("unknown furnace tier `{}`", furnace.tier);
            errors.push(PackError::invalid(&place("blocks", id), message));
        }
    }
    for (_, id, item) in items.iter() {
        let Some(kit) = &item.kit else {
            continue;
        };
        for tier in [&kit.from, &kit.to] {
            if !tiers.contains_key(tier) {
                let message = format!("kit: unknown furnace tier `{tier}`");
                errors.push(PackError::invalid(&place("items", id), message));
            }
        }
    }
}
