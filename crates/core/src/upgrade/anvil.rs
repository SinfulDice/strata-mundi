//! The anvil: an item in its first cell, a material (or a second like item) in the other.
//! With an upgrade chosen, the material pays for its next level; with none, it mends the
//! item. Pure functions: the server applies them, the screen shows them, tests check them.

use super::installed::{count_repair, install, installed, repairs};
use super::stats::max_durability;
use super::{Target, Upgrade, UpgradeLevel, Upgrades};
use crate::content::{Item, ItemId, Registry};
use crate::crafting::{into_cursor, into_inventory};
use crate::id::Identifier;
use crate::inventory::{Click, ItemRules, ItemStack, Window, clear_empty};

/// Cell of the item worked on, and of what pays.
pub const ANVIL_ITEM: usize = 0;
pub const ANVIL_MATERIAL: usize = 1;

/// Why an upgrade cannot go on the item yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocked {
    /// Every slot of its family is taken.
    NoSlot,
    /// It needs this upgrade first.
    Requires(Identifier),
    /// It cannot share the item with this one.
    Excludes(Identifier),
}

/// An upgrade the item could take: its next level and what that costs.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice<'a> {
    /// Index in `Upgrades::list`.
    pub index: u16,
    pub upgrade: &'a Upgrade,
    /// The level it would reach (1 for a new one).
    pub level: u8,
    pub cost: &'a UpgradeLevel,
    pub blocked: Option<Blocked>,
}

/// What taking the anvil's result gives and takes.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub result: ItemStack,
    /// Taken off the material cell.
    pub material: u16,
    /// Experience it costs.
    pub xp: u32,
}

/// The upgrades of the item's slot families that suit it and have a level left.
pub fn choices<'a>(
    stack: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &'a Upgrades,
) -> Vec<Choice<'a>> {
    let Some(item) = items.get(stack.item) else {
        return Vec::new();
    };
    let held = installed(stack);
    upgrades
        .list()
        .iter()
        .enumerate()
        .filter(|(_, upgrade)| item.gear.upgrade_slots.contains(&upgrade.slot))
        .filter(|(_, upgrade)| suits(upgrade, item))
        .filter_map(|(index, upgrade)| {
            let now = held.iter().find(|(id, _)| *id == upgrade.id).map(|h| h.1);
            let level = now.unwrap_or(0).saturating_add(1);
            let cost = upgrade.levels.get(usize::from(level) - 1)?;
            Some(Choice {
                index: u16::try_from(index).ok()?,
                upgrade,
                level,
                cost,
                blocked: blocked(upgrade, now.is_some(), item, &held, upgrades),
            })
        })
        .collect()
}

fn suits(upgrade: &Upgrade, item: &Item) -> bool {
    upgrade.applies_to.is_empty()
        || upgrade.applies_to.iter().any(|target| match target {
            Target::Tool => item.tool.is_some(),
            Target::Weapon => item.gear.weapon.is_some(),
            Target::Armor => item.gear.armor.is_some(),
        })
}

fn blocked(
    upgrade: &Upgrade,
    held_already: bool,
    item: &Item,
    held: &[(Identifier, u8)],
    upgrades: &Upgrades,
) -> Option<Blocked> {
    let holds = |id: &Identifier| held.iter().any(|(other, _)| other == id);
    if let Some(missing) = upgrade.requires.iter().find(|id| !holds(id)) {
        return Some(Blocked::Requires(missing.clone()));
    }
    let clash = held.iter().map(|(id, _)| id).find(|id| {
        upgrade.excludes.contains(id)
            || upgrades
                .get(id)
                .is_some_and(|other| other.excludes.contains(&upgrade.id))
    });
    if let Some(clash) = clash {
        return Some(Blocked::Excludes(clash.clone()));
    }
    let slots = item.gear.upgrade_slots.iter();
    let room = slots.filter(|family| **family == upgrade.slot).count();
    let taken = held
        .iter()
        .filter(|(id, _)| upgrades.get(id).is_some_and(|u| u.slot == upgrade.slot))
        .count();
    (!held_already && taken >= room).then_some(Blocked::NoSlot)
}

/// What the anvil makes of its cells: the chosen upgrade's next level, or a repair.
pub fn anvil_outcome(
    target: Option<&ItemStack>,
    material: Option<&ItemStack>,
    choice: Option<u16>,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Option<Outcome> {
    let target = target.filter(|stack| stack.count == 1)?;
    let material = material?;
    match choice {
        Some(index) => upgrade_outcome(target, material, index, items, upgrades),
        None => repair_outcome(target, material, items, upgrades),
    }
}

fn upgrade_outcome(
    target: &ItemStack,
    material: &ItemStack,
    index: u16,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Option<Outcome> {
    let all = choices(target, items, upgrades);
    let choice = all
        .iter()
        .find(|c| c.index == index && c.blocked.is_none())?;
    let cost = choice.cost;
    if !cost.material.accepts(material.item) || material.count < cost.count {
        return None;
    }
    let mut result = target.clone();
    install(
        &mut result.data,
        &installed(target),
        &choice.upgrade.id,
        choice.level,
    );
    let before = max_durability(target, items, upgrades);
    let after = max_durability(&result, items, upgrades);
    if let (Some(left), Some(before), Some(after)) = (result.durability, before, after) {
        let gained = after.saturating_sub(before);
        result.durability = Some(left.saturating_add(gained).min(after));
    }
    Some(Outcome {
        result,
        material: cost.count,
        xp: cost.xp,
    })
}

/// Mends with the item's repair material (each gives back a share of the uses) or with
/// a second like item without upgrades (both uses added, and a bonus).
fn repair_outcome(
    target: &ItemStack,
    material: &ItemStack,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
) -> Option<Outcome> {
    let full = max_durability(target, items, upgrades)?;
    let left = target.durability?;
    let missing = full.checked_sub(left).filter(|&m| m > 0)?;
    let rules = &upgrades.anvil;
    let (restored, used) = if material.item == target.item {
        let other = material
            .durability
            .filter(|_| installed(material).is_empty())?;
        let bonus = (f32::from(full) * rules.combine_bonus).round() as u16;
        (other.saturating_add(bonus), 1)
    } else {
        let mends = upgrades.repairs.get(&target.item)?;
        if !mends.accepts(material.item) {
            return None;
        }
        let per = ((f32::from(full) * rules.material_share).ceil() as u16).max(1);
        let used = missing.div_ceil(per).min(material.count);
        (per.saturating_mul(used), used)
    };
    let mut result = target.clone();
    result.durability = Some(left.saturating_add(restored.min(missing)));
    let before = repairs(target);
    count_repair(&mut result.data, before);
    Some(Outcome {
        result,
        material: used,
        xp: rules.repair_cost(before),
    })
}

/// A click on the anvil's result: it goes to the hand (or the inventory with a shift
/// click), the item and the material paid leave their cells. `false` if nothing moved.
pub fn take_outcome(
    window: &mut Window<'_>,
    outcome: Outcome,
    click: Click,
    rules: &impl ItemRules,
) -> bool {
    let placed = match click {
        Click::Primary | Click::Secondary => into_cursor(window, outcome.result, rules),
        Click::Transfer => into_inventory(window, outcome.result, rules),
        _ => false,
    };
    if !placed {
        return false;
    }
    let Some(grid) = window.craft.as_deref_mut() else {
        return true;
    };
    if let Some(cell) = grid.slots.slots.get_mut(ANVIL_ITEM) {
        *cell = None;
    }
    if let Some(cell) = grid.slots.slots.get_mut(ANVIL_MATERIAL) {
        if let Some(stack) = cell.as_mut() {
            stack.count = stack.count.saturating_sub(outcome.material);
        }
        clear_empty(cell);
    }
    true
}
