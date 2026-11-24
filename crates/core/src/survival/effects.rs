//! Status effects on a body: bleeding, healing, burning, and later the potions and the
//! spells. One effect is there at most once: given again, the stronger stays (a longer
//! one if they are as strong), never both.

use super::defs::{EffectGrant, StatusEffectDef};
use crate::content::TICKS_PER_SECOND;
use crate::id::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveEffect {
    pub effect: Identifier,
    pub strength: f32,
    pub ticks_left: u32,
    /// Ticks since it began, for what it does every so often.
    #[serde(default)]
    pub age: u32,
}

/// What an effect does this tick.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectPulse {
    Heal(f32),
    Damage { kind: Identifier, amount: f32 },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StatusEffects(pub Vec<ActiveEffect>);

impl StatusEffects {
    /// Gives `grant`; returns whether it changed anything (a weaker one changes nothing).
    pub fn give(&mut self, grant: &EffectGrant) -> bool {
        let ticks = (grant.seconds * TICKS_PER_SECOND as f32).round().max(1.0) as u32;
        let new = ActiveEffect {
            effect: grant.effect.clone(),
            strength: grant.strength,
            ticks_left: ticks,
            age: 0,
        };
        match self.0.iter_mut().find(|e| e.effect == grant.effect) {
            None => {
                self.0.push(new);
                true
            }
            Some(old) => {
                let stronger = new.strength > old.strength
                    || (new.strength == old.strength && new.ticks_left > old.ticks_left);
                if stronger {
                    *old = new;
                }
                stronger
            }
        }
    }

    /// Ends the effects named; returns whether one was there.
    pub fn cure(&mut self, effects: &[Identifier]) -> bool {
        let before = self.0.len();
        self.0.retain(|e| !effects.contains(&e.effect));
        self.0.len() != before
    }

    pub fn has(&self, effect: &Identifier) -> bool {
        self.0.iter().any(|e| &e.effect == effect)
    }

    /// One tick: what each effect does now, then the ended ones go. In water, those that
    /// end in water end. Effects whose pack is gone end at once.
    pub fn tick(
        &mut self,
        defs: &BTreeMap<Identifier, StatusEffectDef>,
        in_water: bool,
    ) -> Vec<EffectPulse> {
        let mut pulses = Vec::new();
        for active in &mut self.0 {
            let Some(def) = defs.get(&active.effect) else {
                active.ticks_left = 0;
                continue;
            };
            if in_water && def.ends_in_water {
                active.ticks_left = 0;
                continue;
            }
            active.age += 1;
            active.ticks_left = active.ticks_left.saturating_sub(1);
            if let Some(heal) = def.heal
                && active.age % u32::from(heal.every) == 0
            {
                pulses.push(EffectPulse::Heal(heal.amount * active.strength));
            }
            if let Some(hurt) = &def.damage
                && active.age % u32::from(hurt.every) == 0
            {
                pulses.push(EffectPulse::Damage {
                    kind: hurt.kind.clone(),
                    amount: hurt.amount * active.strength,
                });
            }
        }
        self.0.retain(|e| e.ticks_left > 0);
        pulses
    }

    /// What the effects give to `stat`: of those giving it, only the strongest counts.
    pub fn stat(&self, defs: &BTreeMap<Identifier, StatusEffectDef>, stat: &Identifier) -> f32 {
        self.0
            .iter()
            .filter_map(|active| {
                let def = defs.get(&active.effect)?;
                let (_, value) = def.stats.iter().find(|(id, _)| id == stat)?;
                Some(value * active.strength)
            })
            .fold(0.0, f32::max)
    }
}
