//! The one computation of what is worn: each armour piece's protection and weight (its
//! upgrades counted), then the bricks of the pieces worn and of the set bonuses reached.
//! Same effect, not cumulative: of the bricks of one kind (same stat, same operation),
//! only the strongest counts, whichever piece or set gives it (two like rings count once).

use super::Worn;
use crate::content::{Item, ItemId, Registry};
use crate::gear::Burden;
use crate::id::Identifier;
use crate::inventory::{ItemStack, PlayerInventory, worn_slots};
use crate::upgrade::{Effect, ItemStats, StatOp, Upgrades, stack_stats, strongest_bricks};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The worn stats the engine reads (the others wait in `WornStats::values` for theirs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WornStat {
    /// Health points added (survival gauges, milestone 8).
    MaxHealth,
    /// Seconds of breath added under water (milestone 8).
    Breath,
    /// Potion slots of a belt (potions come later).
    PotionSlots,
    /// Protection from the cold (temperature, milestone 8).
    Insulation,
    /// Jumps in the air before landing.
    AirJumps,
    /// More than 0: may dash.
    Dash,
    /// More than 0: climbs walls walked into.
    Climb,
}

impl WornStat {
    pub const ALL: [Self; 7] = [
        Self::MaxHealth,
        Self::Breath,
        Self::PotionSlots,
        Self::Insulation,
        Self::AirJumps,
        Self::Dash,
        Self::Climb,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::MaxHealth => "max_health",
            Self::Breath => "breath",
            Self::PotionSlots => "potion_slots",
            Self::Insulation => "insulation",
            Self::AirJumps => "air_jumps",
            Self::Dash => "dash",
            Self::Climb => "climb",
        }
    }
}

/// A set with at least one piece worn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetProgress {
    /// Index in `Worn::sets`.
    pub set: usize,
    /// Different pieces worn, out of `total`.
    pub worn: u8,
    pub total: u8,
    /// The bonus reached (an index in the set's bonuses).
    pub bonus: Option<usize>,
}

/// What the player wears comes to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WornStats {
    /// Of the pieces not broken, set and accessory bricks counted.
    pub protection: u16,
    /// Of every piece, broken or not, lightening counted.
    pub burden: Burden,
    /// Other stats, from 0.
    pub values: BTreeMap<Identifier, f32>,
    pub sets: Vec<SetProgress>,
}

impl WornStats {
    pub fn get(&self, stat: WornStat) -> f32 {
        self.values
            .iter()
            .find(|(id, _)| id.namespace() == "strata" && id.name() == stat.name())
            .map_or(0.0, |(_, value)| *value)
    }

    /// The progress of set `set` (an index in `Worn::sets`).
    pub fn set(&self, set: usize) -> Option<&SetProgress> {
        self.sets.iter().find(|progress| progress.set == set)
    }
}

/// `worn`: each worn stack and its stats (from a `StatsCache`, or `stack_stats`).
pub fn worn_stats<'a>(
    worn: impl IntoIterator<Item = (&'a ItemStack, Arc<ItemStats>)>,
    items: &Registry<ItemId, Item>,
    rules: &Worn,
) -> WornStats {
    let worn: Vec<(&ItemStack, Arc<ItemStats>)> = worn.into_iter().collect();
    let mut totals = WornStats::default();
    let mut bricks: Vec<Effect> = Vec::new();
    for (stack, stats) in &worn {
        if let Some(armor) = &stats.armor {
            totals.burden = totals.burden + armor.burden;
            if !stack.is_broken() {
                totals.protection = totals.protection.saturating_add(armor.protection);
            }
        }
        if stack.is_broken() {
            continue;
        }
        if let Some(item) = items.get(stack.item) {
            bricks.extend(item.gear.worn.iter().cloned());
        }
        bricks.extend(stats.extra.iter().map(|(stat, value)| Effect::Stat {
            stat: stat.clone(),
            op: StatOp::Add,
            value: *value,
        }));
    }
    let whole: Vec<&ItemStack> = worn.iter().map(|(stack, _)| *stack).collect();
    for (index, set) in rules.sets.iter().enumerate() {
        let count = set
            .pieces
            .iter()
            .filter(|piece| whole.iter().any(|s| s.item == **piece && !s.is_broken()))
            .count();
        if count == 0 {
            continue;
        }
        let progress = SetProgress {
            set: index,
            worn: u8::try_from(count).unwrap_or(u8::MAX),
            total: set.size(),
            bonus: set.bonus(u8::try_from(count).unwrap_or(u8::MAX)),
        };
        if let Some(bonus) = progress.bonus.and_then(|b| set.bonuses.get(b)) {
            bricks.extend(bonus.effects.iter().cloned());
        }
        totals.sets.push(progress);
    }
    for effect in strongest_bricks(bricks) {
        apply(&mut totals, &effect);
    }
    totals
}

/// The worn stats of an inventory, worked out without a cache (tests, the wiki).
pub fn inventory_worn_stats(
    inventory: &PlayerInventory,
    items: &Registry<ItemId, Item>,
    upgrades: &Upgrades,
    rules: &Worn,
) -> WornStats {
    let worn = worn_slots()
        .filter_map(|slot| inventory.slots.get(slot))
        .map(|stack| (stack, Arc::new(stack_stats(stack, items, upgrades))));
    worn_stats(worn, items, rules)
}

fn apply(totals: &mut WornStats, effect: &Effect) {
    let Effect::Stat { stat, op, value } = effect else {
        return;
    };
    let (op, value) = (*op, *value);
    let engine = stat.namespace() == "strata";
    match stat.name() {
        "protection" if engine => {
            let points = change(f32::from(totals.protection), op, value).round();
            totals.protection = points.clamp(0.0, f32::from(u16::MAX)) as u16;
        }
        "weight" if engine => {
            let burden = &mut totals.burden;
            for share in [&mut burden.speed, &mut burden.stamina, &mut burden.mana] {
                let changed = match op {
                    StatOp::Add => *share + value / 100.0,
                    StatOp::Percent => change(*share, op, value),
                };
                *share = changed.clamp(0.0, crate::gear::MAX_BURDEN);
            }
        }
        _ => {
            let entry = totals.values.entry(stat.clone()).or_insert(0.0);
            *entry = change(*entry, op, value);
        }
    }
}

fn change(value: f32, op: StatOp, by: f32) -> f32 {
    match op {
        StatOp::Add => value + by,
        StatOp::Percent => value * (1.0 + by / 100.0).max(0.0),
    }
}
