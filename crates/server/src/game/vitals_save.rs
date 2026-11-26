//! The gauges in the player's save: written as they are, read back within their bounds.

use super::Game;
use strata_core::survival::{Health, Needs};
use strata_core::worn::WornStat;

impl Game {
    /// The gauges to save.
    pub fn saved_vitals(&self) -> Option<crate::save::SavedVitals> {
        Some(crate::save::SavedVitals {
            health: self.ecs.healths.get(self.player)?.current,
            needs: self.ecs.needs.get(self.player)?.clone(),
            effects: self
                .ecs
                .effects
                .get(self.player)
                .cloned()
                .unwrap_or_default(),
            temperature: (self.ecs.temperatures.get(self.player))
                .cloned()
                .unwrap_or_default(),
        })
    }

    /// Gauges read from a save, kept within their bounds (a broken value starts full).
    pub fn restore_vitals(&mut self, saved: crate::save::SavedVitals) {
        let rules = &self.rules.survival.rules;
        let full = Needs::full(rules);
        let mut needs = saved.needs;
        let bounded = |value: f32, max: f32, fallback: f32| {
            if value.is_finite() {
                value.clamp(0.0, max)
            } else {
                fallback
            }
        };
        needs.food = bounded(needs.food, rules.hunger.max, full.food);
        needs.saturation = bounded(needs.saturation, needs.food, full.saturation);
        needs.exhaustion = bounded(needs.exhaustion, rules.hunger.per_point, 0.0);
        needs.thirst = bounded(needs.thirst, rules.thirst.max, full.thirst);
        needs.thirst_exhaustion = bounded(needs.thirst_exhaustion, rules.thirst.per_point, 0.0);
        needs.stamina = bounded(needs.stamina, rules.stamina.max, full.stamina);
        let max = rules.health.max + self.equipment().stats.get(WornStat::MaxHealth);
        let mut health = Health::full(max.max(1.0));
        health.current = bounded(saved.health, health.max, health.max);
        if health.is_dead() {
            health.current = health.max;
        }
        self.ecs.healths.insert(self.player, health);
        self.ecs.needs.insert(self.player, needs);
        self.ecs.effects.insert(self.player, saved.effects);
        let temperature = saved.temperature.bounded();
        self.ecs.temperatures.insert(self.player, temperature);
    }
}
