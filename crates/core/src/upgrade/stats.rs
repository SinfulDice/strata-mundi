//! The one computation of what a stack does: its item's values (base and tier, resolved
//! when the packs load), then its upgrades, each brick counted once (the strongest).

use super::defs::{AreaSize, Effect, StatOp};
use super::installed::installed;
use super::{Upgrade, Upgrades};
use crate::content::{Item, ItemId, Registry, ToolDef};
use crate::gear::{Armor, WeaponDef};
use crate::id::Identifier;
use crate::inventory::ItemStack;
use std::collections::BTreeMap;

/// What a stack is as equipment once its upgrades count.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemStats {
    pub tool: Option<ToolDef>,
    pub weapon: Option<WeaponDef>,
    pub armor: Option<Armor>,
    /// Uses of a new one.
    pub durability: Option<u16>,
    /// Areas a hammer may break, smallest first (1×1 first); empty for other items.
    pub areas: Vec<AreaSize>,
    /// The upgrades counted, with their level.
    pub upgrades: Vec<(Identifier, u8)>,
    /// Stats of the packs the engine does not apply yet, from 0.
    pub extra: BTreeMap<Identifier, f32>,
}

/// The stats the engine applies itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineStat {
    Damage,
    MiningSpeed,
    Durability,
    Protection,
    Weight,
}

fn engine_stat(stat: &Identifier) -> Option<EngineStat> {
    if stat.namespace() != "strata" {
        return None;
    }
    Some(match stat.name() {
        "damage" => EngineStat::Damage,
        "mining_speed" => EngineStat::MiningSpeed,
        "durability" => EngineStat::Durability,
        "protection" => EngineStat::Protection,
        "weight" => EngineStat::Weight,
        _ => return None,
    })
}

/// Whether `effect` changes something on `item` (a weapon brick on a pickaxe does not).
pub fn applies(effect: &Effect, item: &Item) -> bool {
    let stat = match effect {
        Effect::Area(..) => return item.tool.is_some(),
        Effect::Stat { stat, .. } => stat,
    };
    match engine_stat(stat) {
        Some(EngineStat::Damage) => item.gear.weapon.is_some(),
        Some(EngineStat::MiningSpeed) => item.tool.is_some(),
        Some(EngineStat::Durability) => item.durability.is_some(),
        Some(EngineStat::Protection | EngineStat::Weight) => item.gear.armor.is_some(),
        None => true,
    }
}

pub fn stack_stats(
    stack: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> ItemStats {
    let Some(item) = items.get(stack.item) else {
        return ItemStats::default();
    };
    let held = held_upgrades(stack, upgrades);
    let mut stats = ItemStats {
        tool: item.tool.clone(),
        weapon: item.gear.weapon,
        armor: item.gear.armor.clone(),
        durability: item.durability,
        areas: areas(&held),
        upgrades: held
            .iter()
            .map(|(u, level)| (u.id.clone(), *level))
            .collect(),
        extra: BTreeMap::new(),
    };
    for effect in strongest(&held) {
        apply(&mut stats, &effect);
    }
    stats
}

/// The tool a stack is, its upgrades counted.
pub fn stack_tool(
    stack: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Option<ToolDef> {
    stack_stats(stack, items, upgrades).tool
}

/// Uses of a new one of this stack, its upgrades counted (only durability bricks are
/// looked at: the wear bar asks for every slot drawn).
pub fn max_durability(
    stack: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Option<u16> {
    let base = items.get(stack.item)?.durability?;
    if stack.data.is_empty() {
        return Some(base);
    }
    let held = held_upgrades(stack, upgrades);
    let uses = strongest(&held)
        .iter()
        .filter_map(|effect| match effect {
            Effect::Stat { stat, op, value }
                if engine_stat(stat) == Some(EngineStat::Durability) =>
            {
                Some((*op, *value))
            }
            _ => None,
        })
        .fold(f32::from(base), |uses, (op, value)| change(uses, op, value));
    Some(uses.round().clamp(1.0, f32::from(u16::MAX)) as u16)
}

/// The known upgrades of the stack, their level within those the packs give.
fn held_upgrades<'a>(stack: &ItemStack, upgrades: &'a Upgrades) -> Vec<(&'a Upgrade, u8)> {
    installed(stack)
        .into_iter()
        .filter_map(|(id, level)| {
            let upgrade = upgrades.get(&id)?;
            let level = level.min(u8::try_from(upgrade.levels.len()).unwrap_or(u8::MAX));
            Some((upgrade, level))
        })
        .collect()
}

/// One brick of each kind, the strongest, of the levels held.
fn strongest(held: &[(&Upgrade, u8)]) -> Vec<Effect> {
    let effects = held
        .iter()
        .filter_map(|(upgrade, level)| upgrade.levels.get(usize::from(*level) - 1))
        .flat_map(|level| level.effects.iter().cloned());
    strongest_bricks(effects)
}

/// Same effect, not cumulative: one brick of each kind (same stat and operation), the
/// strongest; additions before percentages.
pub fn strongest_bricks(effects: impl IntoIterator<Item = Effect>) -> Vec<Effect> {
    let mut kept: Vec<Effect> = Vec::new();
    for effect in effects {
        match kept.iter_mut().find(|other| other.same_kind(&effect)) {
            Some(other) if effect.strength() > other.strength() => *other = effect,
            Some(_) => {}
            None => kept.push(effect),
        }
    }
    kept.sort_by_key(|effect| {
        !matches!(
            effect,
            Effect::Stat {
                op: StatOp::Add,
                ..
            }
        )
    });
    kept
}

/// Every area unlocked by the levels held, and 1×1, smallest first.
fn areas(held: &[(&Upgrade, u8)]) -> Vec<AreaSize> {
    let mut areas: Vec<AreaSize> = held
        .iter()
        .flat_map(|(upgrade, level)| upgrade.levels.iter().take(usize::from(*level)))
        .flat_map(|level| level.effects.iter())
        .filter_map(|effect| match *effect {
            Effect::Area(width, height) => Some(AreaSize { width, height }),
            Effect::Stat { .. } => None,
        })
        .collect();
    if areas.is_empty() {
        return areas;
    }
    areas.push(AreaSize::SINGLE);
    areas.sort_by_key(|area| (area.cells(), area.height, area.width));
    areas.dedup();
    areas
}

fn change(value: f32, op: StatOp, by: f32) -> f32 {
    match op {
        StatOp::Add => value + by,
        StatOp::Percent => value * (1.0 + by / 100.0).max(0.0),
    }
}

fn apply(stats: &mut ItemStats, effect: &Effect) {
    let Effect::Stat { stat, op, value } = effect else {
        return;
    };
    let (op, value) = (*op, *value);
    match engine_stat(stat) {
        Some(EngineStat::Damage) => {
            if let Some(weapon) = &mut stats.weapon {
                weapon.damage = change(weapon.damage, op, value).max(0.0);
            }
        }
        Some(EngineStat::MiningSpeed) => {
            if let Some(tool) = stats.tool.take() {
                let speed = change(tool.speed, op, value);
                stats.tool = Some(ToolDef { speed, ..tool }.clamped());
            }
        }
        Some(EngineStat::Durability) => {
            stats.durability = stats.durability.map(|uses| {
                let more = change(f32::from(uses), op, value).round();
                more.clamp(1.0, f32::from(u16::MAX)) as u16
            });
        }
        Some(EngineStat::Protection) => {
            if let Some(armor) = &mut stats.armor {
                let points = change(f32::from(armor.protection), op, value).round();
                armor.protection = points.clamp(0.0, f32::from(u16::MAX)) as u16;
            }
        }
        Some(EngineStat::Weight) => {
            if let Some(armor) = &mut stats.armor {
                let burden = &mut armor.burden;
                for share in [&mut burden.speed, &mut burden.stamina, &mut burden.mana] {
                    let points = match op {
                        StatOp::Add => *share + value / 100.0,
                        StatOp::Percent => change(*share, op, value),
                    };
                    *share = points.max(0.0);
                }
            }
        }
        None => {
            let entry = stats.extra.entry(stat.clone()).or_insert(0.0);
            *entry = change(*entry, op, value);
        }
    }
}
