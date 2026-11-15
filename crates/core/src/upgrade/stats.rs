//! The one computation of what a stack does: its item's values (base and tier, resolved
//! when the packs load), then its upgrades, each brick counted once (the strongest).

use super::defs::{AreaSize, Effect};
use super::installed::installed;
use super::{Upgrade, Upgrades};
use crate::content::{Item, ItemId, Registry, ToolDef};
use crate::gear::{Armor, WeaponDef};
use crate::id::Identifier;
use crate::inventory::ItemStack;

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
}

pub fn stack_stats(
    stack: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> ItemStats {
    let Some(item) = items.get(stack.item) else {
        return ItemStats::default();
    };
    let held: Vec<(&Upgrade, u8)> = installed(stack)
        .into_iter()
        .filter_map(|(id, level)| {
            let upgrade = upgrades.get(&id)?;
            let level = level.min(u8::try_from(upgrade.levels.len()).unwrap_or(u8::MAX));
            Some((upgrade, level))
        })
        .collect();
    let effects = strongest(
        held.iter()
            .filter_map(|(upgrade, level)| upgrade.levels.get(usize::from(*level) - 1))
            .flat_map(|level| level.effects.iter().copied()),
    );
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
    };
    for effect in effects {
        apply(&mut stats, effect);
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

/// Uses of a new one of this stack, its upgrades counted.
pub fn max_durability(
    stack: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Option<u16> {
    stack_stats(stack, items, upgrades).durability
}

/// One brick of each kind: the strongest of those given.
fn strongest(effects: impl Iterator<Item = Effect>) -> Vec<Effect> {
    let mut kept: Vec<Effect> = Vec::new();
    for effect in effects {
        match kept.iter_mut().find(|other| other.same_kind(&effect)) {
            Some(other) if strength(effect) > strength(*other) => *other = effect,
            Some(_) => {}
            None => kept.push(effect),
        }
    }
    kept
}

fn strength(effect: Effect) -> f32 {
    match effect {
        Effect::Damage(p) | Effect::MiningSpeed(p) | Effect::Durability(p) | Effect::Lighten(p) => {
            p
        }
        Effect::Protection(points) => f32::from(points),
        Effect::Area(w, h) => f32::from(w) * f32::from(h),
    }
}

/// Every area unlocked by the levels held, and 1×1, smallest first.
fn areas(held: &[(&Upgrade, u8)]) -> Vec<AreaSize> {
    let mut areas: Vec<AreaSize> = held
        .iter()
        .flat_map(|(upgrade, level)| upgrade.levels.iter().take(usize::from(*level)))
        .flat_map(|level| level.effects.iter())
        .filter_map(|effect| match *effect {
            Effect::Area(width, height) => Some(AreaSize { width, height }),
            _ => None,
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

fn apply(stats: &mut ItemStats, effect: Effect) {
    let factor = |percent: f32| (1.0 + percent / 100.0).max(0.1);
    match effect {
        Effect::Damage(p) => {
            if let Some(weapon) = &mut stats.weapon {
                weapon.damage *= factor(p);
            }
        }
        Effect::MiningSpeed(p) => {
            if let Some(tool) = stats.tool.take() {
                let speed = tool.speed * factor(p);
                stats.tool = Some(ToolDef { speed, ..tool }.clamped());
            }
        }
        Effect::Durability(p) => {
            stats.durability = stats.durability.map(|uses| {
                let more = (f32::from(uses) * factor(p)).round();
                more.clamp(1.0, f32::from(u16::MAX)) as u16
            });
        }
        Effect::Protection(points) => {
            if let Some(armor) = &mut stats.armor {
                armor.protection = armor.protection.saturating_add(points);
            }
        }
        Effect::Lighten(p) => {
            if let Some(armor) = &mut stats.armor {
                let kept = 1.0 - (p / 100.0).clamp(0.0, 1.0);
                armor.burden.speed *= kept;
                armor.burden.stamina *= kept;
                armor.burden.mana *= kept;
            }
        }
        Effect::Area(..) => {}
    }
}
