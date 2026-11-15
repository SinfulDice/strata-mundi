//! What the anvil wrote in a stack's data: the upgrades it holds (each with its level),
//! how many times it was repaired, and the hammer area chosen.

use super::defs::AreaSize;
use crate::id::Identifier;
use crate::inventory::{DataValue, ItemData, ItemStack};

fn key(name: &'static str) -> Identifier {
    Identifier::builtin("strata", name)
}

/// The upgrades held, in the order they were put in, with their level (1 or more).
pub fn installed(stack: &ItemStack) -> Vec<(Identifier, u8)> {
    let Some(DataValue::List(list)) = stack.data.get(&key("upgrades")) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| match entry {
            DataValue::List(pair) => match pair.as_slice() {
                [DataValue::Id(id), DataValue::Int(level)] => {
                    Some((id.clone(), u8::try_from(*level).ok()?.max(1)))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// `upgrade` at `level`: raised in place if held, added after the others if not.
pub(super) fn install(
    data: &mut ItemData,
    held: &[(Identifier, u8)],
    upgrade: &Identifier,
    level: u8,
) {
    let mut list = held.to_vec();
    match list.iter_mut().find(|(id, _)| id == upgrade) {
        Some((_, old)) => *old = level,
        None => list.push((upgrade.clone(), level)),
    }
    let entries = list
        .into_iter()
        .map(|(id, level)| DataValue::List(vec![DataValue::Id(id), DataValue::Int(level.into())]))
        .collect();
    data.set(key("upgrades"), DataValue::List(entries));
}

/// The stack with `upgrade` at `level` (a named unique given ready, a test).
pub fn with_upgrade(stack: &ItemStack, upgrade: &Identifier, level: u8) -> ItemStack {
    let mut upgraded = stack.clone();
    install(&mut upgraded.data, &installed(stack), upgrade, level.max(1));
    upgraded
}

/// Repairs made at the anvil: each one costs more.
pub fn repairs(stack: &ItemStack) -> u32 {
    match stack.data.get(&key("repairs")) {
        Some(DataValue::Int(count)) => u32::try_from(*count).unwrap_or(0),
        _ => 0,
    }
}

pub(super) fn count_repair(data: &mut ItemData, before: u32) {
    data.set(
        key("repairs"),
        DataValue::Int(before.saturating_add(1).into()),
    );
}

/// The area chosen among those unlocked (`stats.areas`); the largest until one is.
pub fn chosen_area(stack: &ItemStack, areas: &[AreaSize]) -> AreaSize {
    let chosen = match stack.data.get(&key("area")) {
        Some(DataValue::Int(index)) => usize::try_from(*index).ok(),
        _ => None,
    };
    chosen
        .and_then(|index| areas.get(index))
        .or(areas.last())
        .copied()
        .unwrap_or(AreaSize::SINGLE)
}

/// The stack with area `index` of its unlocked ones chosen.
pub fn with_area(stack: &ItemStack, index: usize) -> ItemStack {
    let mut chosen = stack.clone();
    let index = i64::try_from(index).unwrap_or(0);
    chosen.data.set(key("area"), DataValue::Int(index));
    chosen
}
