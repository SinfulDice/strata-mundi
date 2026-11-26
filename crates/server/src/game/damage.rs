//! Harm to any living body, whatever its source: the reduction (armour, resistances,
//! invulnerability) is `survival::take_hit`; a player's equipment and effects defend
//! them. At 0 health a player faints and wakes at the spawn point keeping everything:
//! death (dropped items, tombs) comes with milestone 10.

use super::Game;
use crate::ecs::Entity;
use crate::loader::ColumnLoader;
use glam::DVec3;
use std::sync::Arc;
use strata_core::id::Identifier;
use strata_core::survival::{Defences, Health, Needs, StatusEffects, take_hit};
use strata_core::worn::Equipment;
use strata_protocol::{HurtView, ServerMessage};

impl Game {
    /// Hurts `entity` with `amount` of `kind`; returns the health taken. Creative players
    /// are never hurt.
    pub fn hurt(&mut self, entity: Entity, kind: &Identifier, amount: f32) -> f32 {
        if self.is_creative_entity(entity) {
            return 0.0;
        }
        let survival = Arc::clone(&self.rules.survival);
        let def = survival.damage.get(kind);
        let equipment = self.worn_by(entity);
        let effects = self.ecs.effects.get(entity).cloned();
        let defences = Defences {
            protection: equipment.as_ref().map_or(0, |e| e.stats.protection),
            worn: equipment.as_ref().map(|e| &e.stats.values),
            effects: effects.as_ref(),
        };
        let Some(health) = self.ecs.healths.get_mut(entity) else {
            return 0.0;
        };
        let rules = (&survival.rules, &survival.effects);
        let dealt = take_hit(health, def, amount, &defences, rules);
        if dealt > 0.0 && entity == self.player {
            self.messages.push(ServerMessage::Hurt(HurtView {
                kind: kind.clone(),
                amount: dealt,
                shakes: def.is_none_or(|d| d.shakes),
            }));
        }
        dealt
    }

    /// What `entity` wears (players only, until mobs have equipment).
    pub(super) fn worn_by(&self, entity: Entity) -> Option<Arc<Equipment>> {
        (entity == self.player).then(|| self.equipment())
    }

    /// A player in creative mode (mobs never are).
    pub(super) fn is_creative_entity(&self, entity: Entity) -> bool {
        self.ecs
            .controls
            .get(entity)
            .is_some_and(|c| c.mode == strata_protocol::GameMode::Creative)
    }

    /// 0 health: the player wakes at the spawn point with full gauges, keeping their
    /// inventory (temporary, until death and tombs at milestone 10). Screens close, the
    /// effort and the fall going on are forgotten.
    pub(super) fn faint(&mut self, loader: &ColumnLoader) {
        self.close_window(loader);
        self.leave_boat(self.player);
        let spawn = self.spawn;
        if let Some(body) = self.ecs.bodies.get_mut(self.player) {
            body.position = spawn;
            body.velocity = DVec3::ZERO;
            body.flying = false;
        }
        let rules = &self.rules.survival.rules;
        let max = (self.ecs.healths.get(self.player)).map_or(rules.health.max, |h| h.max);
        self.ecs.healths.insert(self.player, Health::full(max));
        self.ecs.needs.insert(self.player, Needs::full(rules));
        let effects = StatusEffects::default();
        self.ecs.effects.insert(self.player, effects);
        self.fresh_temperature(self.player);
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.using = None;
            control.exertion = Default::default();
            control.fallen = 0.0;
        }
        self.stop_break();
        self.messages.push(ServerMessage::Fainted);
    }
}
