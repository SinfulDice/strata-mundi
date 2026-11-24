//! Harm to the player, whatever its source: lessened by armour (for the kinds it stops)
//! and by resistances (equipment and effects, the strongest counts), then held back by
//! the short invulnerability. At 0 health the player faints and wakes at the spawn point
//! keeping everything: death (dropped items, tombs) comes with milestone 10.

use super::Game;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::id::Identifier;
use strata_core::survival::{Health, Needs, StatusEffects, reduced};
use strata_protocol::{HurtView, ServerMessage};

impl Game {
    /// Hurts the player with `amount` of `kind`; returns the health taken. Creative
    /// players are never hurt.
    pub fn hurt(&mut self, kind: &Identifier, amount: f32) -> f32 {
        if self.is_creative() || amount.is_nan() || amount <= 0.0 {
            return 0.0;
        }
        let survival = &self.rules.survival;
        let def = survival.damage.get(kind);
        let equipment = self.equipment();
        let protection = def.filter(|d| d.armor).map(|_| equipment.stats.protection);
        let resistance = def
            .and_then(|d| d.resistance.as_ref())
            .map_or(0.0, |stat| self.resistance(stat, &equipment.stats.values));
        let amount = reduced(amount, protection, resistance, &survival.rules.armor);
        let piercing = def.is_some_and(|d| d.ignores_invulnerability);
        let shakes = def.is_none_or(|d| d.shakes);
        let invulnerability = survival.rules.invulnerability;
        let Some(health) = self.ecs.healths.get_mut(self.player) else {
            return 0.0;
        };
        let dealt = health.hurt(amount, piercing, invulnerability);
        if dealt > 0.0 {
            self.messages.push(ServerMessage::Hurt(HurtView {
                kind: kind.clone(),
                amount: dealt,
                shakes,
            }));
        }
        dealt
    }

    /// A resistance in percent: what is worn or what an effect gives, the stronger.
    fn resistance(
        &self,
        stat: &Identifier,
        worn: &std::collections::BTreeMap<Identifier, f32>,
    ) -> f32 {
        let worn = worn.get(stat).copied().unwrap_or(0.0);
        let effects = self.ecs.effects.get(self.player);
        let given = effects.map_or(0.0, |e| e.stat(&self.rules.survival.effects, stat));
        worn.max(given)
    }

    /// 0 health: the player wakes at the spawn point with full gauges, keeping their
    /// inventory (temporary, until death and tombs at milestone 10).
    pub(super) fn faint(&mut self, _loader: &ColumnLoader) {
        self.leave_boat(self.player);
        let spawn = self.spawn;
        if let Some(body) = self.ecs.bodies.get_mut(self.player) {
            body.position = spawn;
            body.velocity = DVec3::ZERO;
            body.flying = false;
        }
        let rules = &self.rules.survival.rules;
        let max = self
            .ecs
            .healths
            .get(self.player)
            .map_or(rules.health.max, |h| h.max);
        self.ecs.healths.insert(self.player, Health::full(max));
        self.ecs.needs.insert(self.player, Needs::full(rules));
        self.ecs
            .effects
            .insert(self.player, StatusEffects::default());
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.using = None;
        }
        self.stop_break();
        self.messages.push(ServerMessage::Fainted);
    }
}
