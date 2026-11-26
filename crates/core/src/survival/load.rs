//! Reading `survival.ron`, `damage_types/`, `status_effects/`, `consumables/`, `fills/`
//! and `contacts/`, and checking what they name.

use super::defs::{ConsumeDef, ContactDef, DamageType, EffectGrant, FillDef, StatusEffectDef};
use super::temperature::{HeatKnown, TemperatureRules, load_heat_sources};
use super::{Consumable, Contact, Fill, Survival, SurvivalRules, engine};
use crate::content::{Block, BlockId, FluidLevel, Item, ItemId, ItemTags, Registry};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced, read_ron};
use crate::upgrade::ItemStatDef;
use std::collections::BTreeMap;
use std::path::Path;

/// Kinds of damage the engine deals itself: a pack set that defines damage types must
/// define these (lava and fire come from `contacts/`, named there).
const ENGINE_DAMAGE: [&str; 6] = [
    "fall",
    "starvation",
    "dehydration",
    "drowning",
    "suffocation",
    "burning",
];

pub(crate) struct Known<'a> {
    pub items: &'a Registry<ItemId, Item>,
    pub blocks: &'a Registry<BlockId, Block>,
    pub stats: &'a BTreeMap<Identifier, ItemStatDef>,
    pub tags: &'a ItemTags,
}

pub(crate) fn load_survival(
    packs: &PackSet,
    known: &Known<'_>,
    errors: &mut Vec<PackError>,
) -> Survival {
    let damage = load_damage(packs, known, errors);
    let effects = load_effects(packs, known, &damage, errors);
    let check = Checker {
        known,
        damage: &damage,
        effects: &effects,
    };
    let consumables = packs
        .layered::<ConsumeDef>("consumables", errors)
        .into_iter()
        .filter_map(|(id, sourced)| check.consumable(&id, sourced, errors))
        .collect();
    let fills = packs
        .layered::<FillDef>("fills", errors)
        .into_iter()
        .filter_map(|(id, sourced)| check.fill(&id, &sourced, errors))
        .collect();
    let contacts = load_contacts(packs, &check, errors);
    let rules = last_file(packs, "survival.ron", errors)
        .map_or_else(SurvivalRules::default, SurvivalRules::clamped);
    let temperature = last_file(packs, "temperature.ron", errors)
        .map_or_else(TemperatureRules::default, TemperatureRules::clamped);
    check_temperature(&temperature, known, &damage, errors);
    let heat_known = HeatKnown {
        blocks: known.blocks,
        items: known.items,
        tags: known.tags,
    };
    let heat = load_heat_sources(packs, &heat_known, errors);
    let drink_source = known
        .blocks
        .key(&rules.drinking.source)
        .filter(|&block| is_source(known.blocks, block));
    Survival {
        rules,
        damage,
        effects,
        consumables,
        fills,
        contacts,
        drink_source,
        temperature,
        heat,
    }
}

/// The file `name` of the last pack that has one, read.
fn last_file<T: serde::de::DeserializeOwned>(
    packs: &PackSet,
    name: &str,
    errors: &mut Vec<PackError>,
) -> Option<T> {
    let mut found = None;
    for pack in packs.packs() {
        let path = pack.root.join(name);
        if !path.is_file() {
            continue;
        }
        match read_ron::<T>(&path) {
            Ok(read) => found = Some(read),
            Err(error) => errors.push(error),
        }
    }
    found
}

/// The damage kinds and stats `temperature.ron` names exist (when the packs define any).
fn check_temperature(
    rules: &TemperatureRules,
    known: &Known<'_>,
    damage: &BTreeMap<Identifier, DamageType>,
    errors: &mut Vec<PackError>,
) {
    let path = Path::new("temperature.ron");
    for harm in [&rules.freezing, &rules.scorching] {
        if !damage.is_empty() && !damage.contains_key(&harm.damage) {
            let message = format!("unknown damage `{}`", harm.damage);
            errors.push(PackError::invalid(path, message));
        }
    }
    for stat in [&rules.clothing.cold_stat, &rules.clothing.heat_stat] {
        if !known.stats.is_empty() && !known.stats.contains_key(stat) {
            errors.push(PackError::invalid(path, format!("unknown stat `{stat}`")));
        }
    }
}

fn load_damage(
    packs: &PackSet,
    known: &Known<'_>,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, DamageType> {
    let mut damage = BTreeMap::new();
    for (id, Sourced { value, source }) in packs.layered::<DamageType>("damage_types", errors) {
        if let Some(stat) = &value.resistance
            && !known.stats.contains_key(stat)
        {
            errors.push(PackError::invalid(
                &source,
                format!("unknown stat `{stat}`"),
            ));
        }
        damage.insert(id, value);
    }
    if !damage.is_empty() {
        for name in ENGINE_DAMAGE {
            if !damage.contains_key(&engine(name)) {
                let path = Path::new("damage_types").join(format!("{name}.ron"));
                errors.push(PackError::invalid(&path, "missing: the engine deals it"));
            }
        }
    }
    damage
}

fn load_effects(
    packs: &PackSet,
    known: &Known<'_>,
    damage: &BTreeMap<Identifier, DamageType>,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, StatusEffectDef> {
    let mut effects = BTreeMap::new();
    let layered = packs.layered::<StatusEffectDef>("status_effects", errors);
    for (id, Sourced { mut value, source }) in layered {
        if let Some(hurt) = &value.damage
            && !damage.contains_key(&hurt.kind)
        {
            errors.push(PackError::invalid(
                &source,
                format!("unknown damage `{}`", hurt.kind),
            ));
            value.damage = None;
        }
        for (_, value) in value.stats.iter_mut() {
            *value = if value.is_finite() { *value } else { 0.0 };
        }
        value.stats.retain(|(stat, _)| {
            let known_stat = known.stats.contains_key(stat);
            if !known_stat {
                errors.push(PackError::invalid(
                    &source,
                    format!("unknown stat `{stat}`"),
                ));
            }
            known_stat
        });
        for periodic in value.heal.iter_mut() {
            periodic.every = periodic.every.max(1);
        }
        for periodic in value.damage.iter_mut() {
            periodic.every = periodic.every.max(1);
        }
        effects.insert(id, value);
    }
    effects
}

fn load_contacts(
    packs: &PackSet,
    check: &Checker<'_>,
    errors: &mut Vec<PackError>,
) -> Vec<Option<Contact>> {
    let blocks = check.known.blocks;
    let mut contacts = vec![None; blocks.len()];
    for (id, Sourced { value, source }) in packs.layered::<ContactDef>("contacts", errors) {
        let Some(block) = blocks.key(&id) else {
            errors.push(PackError::invalid(&source, format!("unknown block `{id}`")));
            continue;
        };
        if !check.damage.contains_key(&value.damage) {
            let message = format!("unknown damage `{}`", value.damage);
            errors.push(PackError::invalid(&source, message));
            continue;
        }
        let effect = value
            .effect
            .and_then(|grant| check.grant(grant, &source, errors));
        let contact = Contact {
            damage: value.damage,
            amount: finite(value.amount),
            every: value.every.max(1),
            effect,
        };
        // Every block of the family (a lit or turned variant) hurts the same.
        for (key, _, other) in blocks.iter() {
            if (key == block || other.family(key) == block)
                && let Some(slot) = contacts.get_mut(usize::from(key.0))
            {
                *slot = Some(contact.clone());
            }
        }
    }
    contacts
}

fn is_source(blocks: &Registry<BlockId, Block>, block: BlockId) -> bool {
    blocks
        .get(block)
        .and_then(|b| b.fluid)
        .is_some_and(|state| state.level == FluidLevel::Source && state.source == block)
}

fn finite(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

struct Checker<'a> {
    known: &'a Known<'a>,
    damage: &'a BTreeMap<Identifier, DamageType>,
    effects: &'a BTreeMap<Identifier, StatusEffectDef>,
}

impl Checker<'_> {
    fn item(&self, id: &Identifier, source: &Path, errors: &mut Vec<PackError>) -> Option<ItemId> {
        let item = self.known.items.key(id);
        if item.is_none() {
            errors.push(PackError::invalid(source, format!("unknown item `{id}`")));
        }
        item
    }

    fn grant(
        &self,
        grant: EffectGrant,
        source: &Path,
        errors: &mut Vec<PackError>,
    ) -> Option<EffectGrant> {
        if !self.effects.contains_key(&grant.effect) {
            let message = format!("unknown status effect `{}`", grant.effect);
            errors.push(PackError::invalid(source, message));
            return None;
        }
        Some(EffectGrant {
            seconds: finite(grant.seconds).min(3600.0),
            strength: finite(grant.strength).min(100.0),
            ..grant
        })
    }

    fn consumable(
        &self,
        id: &Identifier,
        Sourced { value, source }: Sourced<ConsumeDef>,
        errors: &mut Vec<PackError>,
    ) -> Option<(ItemId, Consumable)> {
        let item = self.item(id, &source, errors)?;
        let stacks = self.known.items.get(item).is_some_and(|i| i.max_stack > 1);
        if value.sips.is_some() && stacks {
            let message = format!("`{id}` has sips but stacks: sips need `max_stack: 1`");
            errors.push(PackError::invalid(&source, message));
            return None;
        }
        let leaves = match &value.leaves {
            Some(left) => Some(self.item(left, &source, errors)?),
            None => None,
        };
        let effects = (value.effects.into_iter())
            .filter_map(|grant| self.grant(grant, &source, errors))
            .collect();
        for cured in &value.cures {
            if !self.effects.contains_key(cured) {
                let message = format!("unknown status effect `{cured}`");
                errors.push(PackError::invalid(&source, message));
            }
        }
        let number = |value: f32| if value.is_finite() { value } else { 0.0 };
        Some((
            item,
            Consumable {
                ticks: value.ticks.clamp(1, 200),
                food: number(value.food),
                saturation: finite(value.saturation),
                thirst: number(value.thirst),
                effects,
                cures: value.cures,
                leaves,
                sips: value.sips.filter(|&sips| sips > 1),
                sound: value.sound,
                always: value.always,
            },
        ))
    }

    fn fill(
        &self,
        id: &Identifier,
        Sourced { value, source }: &Sourced<FillDef>,
        errors: &mut Vec<PackError>,
    ) -> Option<(ItemId, Fill)> {
        let item = self.item(id, source, errors)?;
        let fresh = self.item(&value.fresh, source, errors)?;
        let salty = self.item(&value.salty, source, errors)?;
        let blocks = self.known.blocks;
        let liquid = blocks.key(&value.liquid).filter(|&b| is_source(blocks, b));
        if liquid.is_none() {
            let message = format!("`{}` is not a liquid source", value.liquid);
            errors.push(PackError::invalid(source, message));
        }
        Some((
            item,
            Fill {
                liquid: liquid?,
                fresh,
                salty,
            },
        ))
    }
}
