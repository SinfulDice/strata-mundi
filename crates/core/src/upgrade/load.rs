//! Reading `upgrades/` and `anvil.ron`, with every unknown name reported with its file.

use super::defs::{AnvilRules, Effect, UpgradeDef};
use super::{Upgrade, UpgradeLevel, Upgrades};
use crate::crafting::{Names, ingredient};
use crate::gear::UpgradeSlotFamily;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced, read_ron};
use std::collections::BTreeMap;
use std::path::Path;

/// More levels would not fit the anvil's list.
const MAX_LEVELS: usize = 8;
/// Bounds of a percent brick: more than ten times would break every balance.
const MAX_PERCENT: f32 = 1000.0;
/// Widest hammer area a side.
pub const MAX_AREA: u8 = 9;

pub(crate) fn load_upgrades(
    packs: &PackSet,
    names: &Names<'_>,
    families: &BTreeMap<Identifier, UpgradeSlotFamily>,
    errors: &mut Vec<PackError>,
) -> Upgrades {
    let list = packs
        .layered::<UpgradeDef>("upgrades", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            upgrade(id, value, names, families, &source, errors)
        })
        .collect();
    let repairs = names
        .items
        .iter()
        .filter_map(|(key, id, item)| {
            let def = item.gear.repair.clone()?;
            let source = Path::new("items").join(format!("{id}"));
            Some((key, ingredient(def, names, &source, errors)?))
        })
        .collect();
    Upgrades::new(list, repairs, anvil_rules(packs, errors))
}

fn upgrade(
    id: Identifier,
    def: UpgradeDef,
    names: &Names<'_>,
    families: &BTreeMap<Identifier, UpgradeSlotFamily>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<Upgrade> {
    if !families.contains_key(&def.slot) {
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
            Some(UpgradeLevel {
                effects: level.effects.into_iter().map(bounded).collect(),
                xp: level.xp,
                material: ingredient(level.material, names, source, errors)?,
                count: level.count.clamp(1, 64),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Upgrade {
        id,
        slot: def.slot,
        applies_to: def.applies_to,
        levels,
        requires: def.requires,
        excludes: def.excludes,
    })
}

fn bounded(effect: Effect) -> Effect {
    let percent = |value: f32| {
        if value.is_finite() {
            value.clamp(-90.0, MAX_PERCENT)
        } else {
            0.0
        }
    };
    match effect {
        Effect::Damage(p) => Effect::Damage(percent(p)),
        Effect::MiningSpeed(p) => Effect::MiningSpeed(percent(p)),
        Effect::Durability(p) => Effect::Durability(percent(p)),
        Effect::Lighten(p) => Effect::Lighten(percent(p).clamp(0.0, 100.0)),
        Effect::Protection(points) => Effect::Protection(points.min(100)),
        Effect::Area(w, h) => Effect::Area(w.clamp(1, MAX_AREA), h.clamp(1, MAX_AREA)),
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
