//! One hit on any body: lessened by armour (for the kinds it stops) and by a resistance,
//! then held back by the short invulnerability. Free functions, so players and mobs share
//! them.

use super::defs::{DamageType, StatusEffectDef};
use super::effects::StatusEffects;
use super::health::{Health, reduced};
use super::rules::SurvivalRules;
use crate::id::Identifier;
use std::collections::BTreeMap;

/// What protects a body: its armour points and the stats it wears.
#[derive(Debug, Clone, Copy, Default)]
pub struct Defences<'a> {
    pub protection: u16,
    pub worn: Option<&'a BTreeMap<Identifier, f32>>,
    pub effects: Option<&'a StatusEffects>,
}

/// The health a hit of `amount` takes from `health` (0 when held back). An unknown kind
/// is plain harm: no armour, no resistance.
pub fn take_hit(
    health: &mut Health,
    kind: Option<&DamageType>,
    amount: f32,
    defences: &Defences<'_>,
    (rules, effect_defs): (&SurvivalRules, &BTreeMap<Identifier, StatusEffectDef>),
) -> f32 {
    if amount.is_nan() || amount <= 0.0 {
        return 0.0;
    }
    let protection = kind.filter(|k| k.armor).map(|_| defences.protection);
    let resistance = (kind.and_then(|k| k.resistance.as_ref()))
        .map_or(0.0, |stat| resistance(stat, defences, effect_defs));
    let amount = reduced(amount, protection, resistance, &rules.armor);
    let piercing = kind.is_some_and(|k| k.ignores_invulnerability);
    health.hurt(amount, piercing, rules.invulnerability)
}

/// A resistance in percent: what is worn or what an effect gives, the stronger of the two.
pub fn resistance(
    stat: &Identifier,
    defences: &Defences<'_>,
    effect_defs: &BTreeMap<Identifier, StatusEffectDef>,
) -> f32 {
    let worn = (defences.worn.and_then(|worn| worn.get(stat)))
        .copied()
        .unwrap_or(0.0);
    let given = defences
        .effects
        .map_or(0.0, |effects| effects.stat(effect_defs, stat));
    worn.max(given)
}
