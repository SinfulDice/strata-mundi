//! Reading `upgrades/` and `anvil.ron`, with every unknown name reported with its file.

use super::defs::{AnvilRules, Effect, ItemStatDef, UpgradeDef};
use super::{Upgrade, UpgradeLevel, Upgrades};
use crate::crafting::{Ingredient, IngredientDef, Names, ingredient};
use crate::gear::UpgradeSlotFamily;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced, read_ron};
use std::collections::BTreeMap;
use std::path::Path;

/// A bound against pack typos: no upgrade needs more levels than this.
const MAX_LEVELS: usize = 8;
/// Widest hammer area a side.
pub const MAX_AREA: u8 = 9;

pub(crate) fn load_upgrades(
    packs: &PackSet,
    names: &Names<'_>,
    families: &BTreeMap<Identifier, UpgradeSlotFamily>,
    errors: &mut Vec<PackError>,
) -> Upgrades {
    let stats: BTreeMap<Identifier, ItemStatDef> = packs
        .layered::<ItemStatDef>("item_stats", errors)
        .into_iter()
        .map(|(id, Sourced { value, .. })| (id, value))
        .collect();
    let known = Known {
        names,
        families,
        stats: &stats,
    };
    let list: Vec<Upgrade> = packs
        .layered::<UpgradeDef>("upgrades", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| upgrade(id, value, &known, &source, errors))
        .collect();
    check_links(&list, errors);
    let repairs = names
        .items
        .iter()
        .filter_map(|(key, id, item)| {
            let def = item.gear.repair.clone()?;
            let source = Path::new("items").join(format!("{id}"));
            Some((key, ingredient(def, names, &source, errors)?))
        })
        .collect();
    let mut upgrades = Upgrades::new(list, repairs, anvil_rules(packs, errors));
    upgrades.stats = stats;
    upgrades
}

/// What upgrades may refer to.
struct Known<'a> {
    names: &'a Names<'a>,
    families: &'a BTreeMap<Identifier, UpgradeSlotFamily>,
    stats: &'a BTreeMap<Identifier, ItemStatDef>,
}

fn upgrade(
    id: Identifier,
    def: UpgradeDef,
    known: &Known<'_>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<Upgrade> {
    if !known.families.contains_key(&def.slot) {
        let message = format!("unknown upgrade slot `{}`", def.slot);
        errors.push(PackError::invalid(source, message));
        return None;
    }
    if def.levels.is_empty() || def.levels.len() > MAX_LEVELS {
        let message = format!("an upgrade needs 1 to {MAX_LEVELS} levels");
        errors.push(PackError::invalid(source, message));
        return None;
    }
    let levels = def
        .levels
        .into_iter()
        .map(|level| {
            let effects = level.effects.into_iter();
            Some(UpgradeLevel {
                effects: effects
                    .filter_map(|e| bounded(e, known.stats, source, errors))
                    .collect(),
                xp: level.xp,
                material: ingredient(level.material, known.names, source, errors)?,
                count: level.count.clamp(1, 64),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Upgrade {
        id,
        slot: def.slot,
        applies_to: targets(def.applies_to, known.names, source, errors),
        levels,
        requires: def.requires,
        excludes: def.excludes,
    })
}

/// The items of the tags named, together; `None` (any item) when none is named.
fn targets(
    tags: Vec<Identifier>,
    names: &Names<'_>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<Ingredient> {
    if tags.is_empty() {
        return None;
    }
    let items = tags
        .into_iter()
        .filter_map(|tag| ingredient(IngredientDef::Tag(tag), names, source, errors))
        .flat_map(|found| found.items().to_vec())
        .collect();
    Some(Ingredient::new(items, None))
}

/// A stat brick of a known stat, its value within the stat's bounds; an area within
/// `MAX_AREA`.
pub(crate) fn bounded(
    effect: Effect,
    stats: &BTreeMap<Identifier, ItemStatDef>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<Effect> {
    match effect {
        Effect::Stat { stat, op, value } => {
            let Some(bounds) = stats.get(&stat) else {
                errors.push(PackError::invalid(source, format!("unknown stat `{stat}`")));
                return None;
            };
            let value = if value.is_finite() { value } else { 0.0 };
            let value = value.clamp(bounds.min, bounds.max.max(bounds.min));
            Some(Effect::Stat { stat, op, value })
        }
        Effect::Area(w, h) => Some(Effect::Area(w.clamp(1, MAX_AREA), h.clamp(1, MAX_AREA))),
    }
}

/// Requirements and exclusions must name upgrades that exist.
pub(super) fn check_links(list: &[Upgrade], errors: &mut Vec<PackError>) {
    for upgrade in list {
        let named = upgrade.requires.iter().chain(&upgrade.excludes);
        for other in named.filter(|id| !list.iter().any(|u| u.id == **id)) {
            let source = Path::new("upgrades").join(format!("{}", upgrade.id));
            let message = format!("unknown upgrade `{other}` in requires or excludes");
            errors.push(PackError::invalid(&source, message));
        }
    }
}

/// The last pack that has `anvil.ron` wins.
fn anvil_rules(packs: &PackSet, errors: &mut Vec<PackError>) -> AnvilRules {
    let mut rules = AnvilRules::default();
    for pack in packs.packs() {
        let path = pack.root.join("anvil.ron");
        if !path.is_file() {
            continue;
        }
        match read_ron::<AnvilRules>(&path) {
            Ok(read) => rules = read.clamped(),
            Err(error) => errors.push(error),
        }
    }
    rules
}
