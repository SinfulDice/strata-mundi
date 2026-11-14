//! Loot tables (`loot_tables/<name>.ron`): what a broken block gives. A block without a
//! table gives its own item.

use super::registry::{BlockId, ItemId, Registry};
use super::{Block, Item, ToolDef};
use crate::audio::Rng;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

const MAX_ROLLS: u8 = 16;
const MAX_COUNT: u16 = 1024;

/// A table as written in the pack: its pools are rolled one after the other.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "LootTable", deny_unknown_fields)]
pub struct LootTableDef {
    pub pools: Vec<PoolDef>,
}

/// Picks `rolls` entries, each with a chance proportional to its weight.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Pool", deny_unknown_fields)]
pub struct PoolDef {
    #[serde(default = "one_roll")]
    pub rolls: u8,
    /// The whole pool is skipped unless all hold.
    #[serde(default)]
    pub conditions: Vec<LootCondition>,
    pub entries: Vec<EntryDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum EntryDef {
    /// Between `count.0` and `count.1` (both included) of `item`.
    Item {
        item: Identifier,
        #[serde(default = "one_weight")]
        weight: u32,
        #[serde(default = "single")]
        count: (u16, u16),
        #[serde(default)]
        conditions: Vec<LootCondition>,
    },
    /// The broken block's own item.
    Itself {
        #[serde(default = "one_weight")]
        weight: u32,
        #[serde(default = "single")]
        count: (u16, u16),
        #[serde(default)]
        conditions: Vec<LootCondition>,
    },
    /// Nothing at all: its weight is the chance of an empty roll.
    Nothing {
        #[serde(default = "one_weight")]
        weight: u32,
    },
}

/// When a pool or an entry counts. Silk touch and enchantments will join them.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum LootCondition {
    /// The tool breaking the block is of this kind.
    Tool(Identifier),
    /// The inner condition does not hold.
    Not(Box<LootCondition>),
    /// Holds with this probability, from 0 to 1.
    Chance(f32),
}

fn one_roll() -> u8 {
    1
}

fn one_weight() -> u32 {
    1
}

fn single() -> (u16, u16) {
    (1, 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootDrop {
    Item(ItemId),
    Itself,
    Nothing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LootEntry {
    pub drop: LootDrop,
    pub weight: u32,
    pub count: (u16, u16),
    pub conditions: Vec<LootCondition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LootPool {
    pub rolls: u8,
    pub conditions: Vec<LootCondition>,
    pub entries: Vec<LootEntry>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LootTable {
    pub pools: Vec<LootPool>,
}

/// What the roll knows about the break.
#[derive(Debug, Clone, Copy)]
pub struct LootContext<'a> {
    pub tool: Option<&'a ToolDef>,
    /// The broken block's item, for `Itself`.
    pub itself: Option<ItemId>,
}

impl LootCondition {
    fn holds(&self, context: &LootContext<'_>, rng: &mut Rng) -> bool {
        match self {
            Self::Tool(kind) => context.tool.is_some_and(|tool| tool.is(kind)),
            Self::Not(inner) => !inner.holds(context, rng),
            Self::Chance(chance) => rng.unit() < *chance,
        }
    }
}

fn all_hold(conditions: &[LootCondition], context: &LootContext<'_>, rng: &mut Rng) -> bool {
    conditions.iter().all(|c| c.holds(context, rng))
}

impl LootTable {
    /// The default: one of the block's own item.
    pub fn itself() -> Self {
        Self {
            pools: vec![LootPool {
                rolls: 1,
                conditions: Vec::new(),
                entries: vec![LootEntry {
                    drop: LootDrop::Itself,
                    weight: 1,
                    count: (1, 1),
                    conditions: Vec::new(),
                }],
            }],
        }
    }

    /// Items and counts given, in pool order (an item may come several times).
    pub fn roll(&self, context: &LootContext<'_>, rng: &mut Rng) -> Vec<(ItemId, u16)> {
        let mut drops = Vec::new();
        for pool in &self.pools {
            if !all_hold(&pool.conditions, context, rng) {
                continue;
            }
            for _ in 0..pool.rolls {
                drops.extend(pool.roll_once(context, rng));
            }
        }
        drops
    }

    /// Items any roll may give (the wiki lists them); `Itself` stands for `itself`.
    pub fn possible_items(&self, itself: Option<ItemId>) -> Vec<ItemId> {
        let mut items: Vec<ItemId> = self
            .pools
            .iter()
            .flat_map(|pool| &pool.entries)
            .filter_map(|entry| match entry.drop {
                LootDrop::Item(item) => Some(item),
                LootDrop::Itself => itself,
                LootDrop::Nothing => None,
            })
            .collect();
        items.sort();
        items.dedup();
        items
    }
}

impl LootPool {
    fn roll_once(&self, context: &LootContext<'_>, rng: &mut Rng) -> Option<(ItemId, u16)> {
        let open: Vec<&LootEntry> = self
            .entries
            .iter()
            .filter(|entry| entry.weight > 0 && all_hold(&entry.conditions, context, rng))
            .collect();
        let total: u64 = open.iter().map(|entry| u64::from(entry.weight)).sum();
        if total == 0 {
            return None;
        }
        let mut pick = rng.next_u64() % total;
        let entry = open.into_iter().find(|entry| {
            let weight = u64::from(entry.weight);
            let found = pick < weight;
            pick = pick.saturating_sub(weight);
            found
        })?;
        let item = match entry.drop {
            LootDrop::Item(item) => item,
            LootDrop::Itself => context.itself?,
            LootDrop::Nothing => return None,
        };
        let (low, high) = entry.count;
        let span = u64::from(high - low) + 1;
        let count = low + (rng.next_u64() % span) as u16;
        (count > 0).then_some((item, count))
    }
}

pub(super) fn load_loot_tables(
    packs: &PackSet,
    items: &Registry<ItemId, Item>,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, LootTable> {
    packs
        .layered::<LootTableDef>("loot_tables", errors)
        .into_iter()
        .map(|(id, Sourced { value, source })| {
            let pools = value
                .pools
                .into_iter()
                .map(|pool| resolve_pool(pool, items, &source, errors))
                .collect();
            (id, LootTable { pools })
        })
        .collect()
}

fn resolve_pool(
    pool: PoolDef,
    items: &Registry<ItemId, Item>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> LootPool {
    if pool.rolls > MAX_ROLLS {
        errors.push(PackError::invalid(
            source,
            format!("`rolls` is {}, the maximum is {MAX_ROLLS}", pool.rolls),
        ));
    }
    check_conditions(&pool.conditions, source, errors);
    let entries = pool
        .entries
        .into_iter()
        .filter_map(|entry| resolve_entry(entry, items, source, errors))
        .collect();
    LootPool {
        rolls: pool.rolls.min(MAX_ROLLS),
        conditions: pool.conditions,
        entries,
    }
}

fn resolve_entry(
    entry: EntryDef,
    items: &Registry<ItemId, Item>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<LootEntry> {
    let (drop, weight, count, conditions) = match entry {
        EntryDef::Item {
            item,
            weight,
            count,
            conditions,
        } => {
            let Some(key) = items.key(&item) else {
                errors.push(PackError::invalid(source, format!("unknown item `{item}`")));
                return None;
            };
            (LootDrop::Item(key), weight, count, conditions)
        }
        EntryDef::Itself {
            weight,
            count,
            conditions,
        } => (LootDrop::Itself, weight, count, conditions),
        EntryDef::Nothing { weight } => (LootDrop::Nothing, weight, (0, 0), Vec::new()),
    };
    check_conditions(&conditions, source, errors);
    let (low, high) = count;
    if low > high || high > MAX_COUNT {
        errors.push(PackError::invalid(
            source,
            format!("`count` must be (min, max) with min ≤ max ≤ {MAX_COUNT}"),
        ));
    }
    let high = high.min(MAX_COUNT);
    Some(LootEntry {
        drop,
        weight,
        count: (low.min(high), high),
        conditions,
    })
}

fn check_conditions(conditions: &[LootCondition], source: &Path, errors: &mut Vec<PackError>) {
    for condition in conditions {
        match condition {
            LootCondition::Chance(chance) if !(0.0..=1.0).contains(chance) => {
                errors.push(PackError::invalid(
                    source,
                    "`Chance` must be between 0 and 1",
                ));
            }
            LootCondition::Not(inner) => {
                check_conditions(std::slice::from_ref(inner), source, errors);
            }
            _ => {}
        }
    }
}

/// Every tool kind the conditions name (each needs a translated name).
pub(super) fn condition_kinds(table: &LootTable) -> Vec<&Identifier> {
    fn walk<'a>(condition: &'a LootCondition, out: &mut Vec<&'a Identifier>) {
        match condition {
            LootCondition::Tool(kind) => out.push(kind),
            LootCondition::Not(inner) => walk(inner, out),
            LootCondition::Chance(_) => {}
        }
    }
    let mut kinds = Vec::new();
    for pool in &table.pools {
        let entries = pool.entries.iter().flat_map(|e| &e.conditions);
        for condition in pool.conditions.iter().chain(entries) {
            walk(condition, &mut kinds);
        }
    }
    kinds
}

/// Blocks naming a table no pack defines.
pub(super) fn check_loot_names(
    blocks: &Registry<BlockId, Block>,
    tables: &BTreeMap<Identifier, LootTable>,
    errors: &mut Vec<PackError>,
) {
    let mut missing: Vec<(&Identifier, &Identifier)> = blocks
        .iter()
        .filter_map(|(_, id, block)| Some((id, block.loot.as_ref()?)))
        .filter(|(_, table)| !tables.contains_key(*table))
        .collect();
    missing.dedup_by(|a, b| a.1 == b.1);
    for (block, table) in missing {
        errors.push(PackError::invalid(
            std::path::Path::new("blocks"),
            format!("`{block}`: unknown loot table `{table}`"),
        ));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "loot_tests.rs"]
mod tests;
