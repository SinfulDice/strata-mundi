//! Reading `sets/` and `mobility.ron`, and checking the bricks items give when worn.

use super::{GearSet, MobilityRules, SetBonus, SetDef, Worn};
use crate::content::{Item, ItemId, Registry};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced, read_ron};
use crate::upgrade::{ItemStatDef, bounded};
use std::collections::BTreeMap;
use std::path::Path;

const MAX_SET_PIECES: usize = 8;

pub(crate) fn load_worn(
    packs: &PackSet,
    items: &Registry<ItemId, Item>,
    stats: &BTreeMap<Identifier, ItemStatDef>,
    errors: &mut Vec<PackError>,
) -> Worn {
    let sets = packs
        .layered::<SetDef>("sets", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            gear_set(id, value, items, stats, &source, errors)
        })
        .collect();
    Worn {
        sets,
        mobility: mobility_rules(packs, errors),
    }
}

fn gear_set(
    id: Identifier,
    def: SetDef,
    items: &Registry<ItemId, Item>,
    stats: &BTreeMap<Identifier, ItemStatDef>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<GearSet> {
    let mut pieces: Vec<ItemId> = Vec::new();
    for name in &def.pieces {
        match items.key(name) {
            Some(item) if items.get(item).is_some_and(|i| i.equip.is_some()) => {
                if !pieces.contains(&item) {
                    pieces.push(item);
                }
            }
            Some(_) => errors.push(PackError::invalid(source, format!("`{name}` is not worn"))),
            None => errors.push(PackError::invalid(source, format!("unknown item `{name}`"))),
        }
    }
    if pieces.len() < 2 || pieces.len() > MAX_SET_PIECES {
        let message = format!("a set needs 2 to {MAX_SET_PIECES} pieces");
        errors.push(PackError::invalid(source, message));
        return None;
    }
    let mut bonuses: Vec<SetBonus> = def
        .bonuses
        .into_iter()
        .filter(|bonus| (1..=pieces.len()).contains(&usize::from(bonus.pieces)))
        .map(|bonus| SetBonus {
            pieces: bonus.pieces,
            effects: bonus
                .effects
                .into_iter()
                .filter_map(|e| bounded(e, stats, source, errors))
                .collect(),
        })
        .collect();
    bonuses.sort_by_key(|bonus| bonus.pieces);
    bonuses.dedup_by_key(|bonus| bonus.pieces);
    Some(GearSet {
        id,
        pieces,
        bonuses,
    })
}

/// The last pack that has `mobility.ron` wins.
fn mobility_rules(packs: &PackSet, errors: &mut Vec<PackError>) -> MobilityRules {
    let mut rules = MobilityRules::default();
    for pack in packs.packs() {
        let path = pack.root.join("mobility.ron");
        if !path.is_file() {
            continue;
        }
        match read_ron::<MobilityRules>(&path) {
            Ok(read) => rules = read.clamped(),
            Err(error) => errors.push(error),
        }
    }
    rules
}

/// Keeps the worn bricks of known stats, within their bounds.
pub(crate) fn bound_worn_effects(
    items: &mut Registry<ItemId, Item>,
    stats: &BTreeMap<Identifier, ItemStatDef>,
    errors: &mut Vec<PackError>,
) {
    let keys: Vec<(ItemId, Identifier)> = items
        .iter()
        .filter(|(_, _, item)| !item.gear.worn.is_empty())
        .map(|(key, id, _)| (key, id.clone()))
        .collect();
    for (key, id) in keys {
        let Some(item) = items.get_mut(key) else {
            continue;
        };
        let source = Path::new("items").join(format!("{id}"));
        let effects = std::mem::take(&mut item.gear.worn);
        item.gear.worn = effects
            .into_iter()
            .filter_map(|e| bounded(e, stats, &source, errors))
            .collect();
    }
}
