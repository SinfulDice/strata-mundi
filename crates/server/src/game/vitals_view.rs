//! The gauges as the client sees them: sprint allowed, the HUD's view, sent only when it
//! changes (stamina in steps, breath in half seconds, the item being used).

use super::Game;
use strata_core::content::TICKS_PER_SECOND;
use strata_core::survival::{Needs, StatusEffects, breath_ticks};
use strata_core::worn::WornStat;
use strata_protocol::{EffectView, ServerMessage, TemperatureView, VitalsView};

/// Stamina is sent in steps of this many points.
const STAMINA_STEP: f32 = 5.0;
/// Breath and the time left of a use are sent in steps of this many ticks.
const TICK_STEP: u16 = 5;

impl Game {
    /// Sprinting needs stamina and food; the client's prediction is told with the gauges.
    pub(super) fn refresh_sprint(&mut self) {
        let creative = self.is_creative();
        let rules = &self.rules.survival.rules;
        let options = self.survival_options;
        let allowed = creative
            || self
                .ecs
                .needs
                .get(self.player)
                .is_none_or(|needs| needs.can_sprint(rules, &options));
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.movement.can_sprint = allowed;
        }
    }

    /// The gauges as the HUD shows them.
    pub fn vitals_view(&self) -> Option<VitalsView> {
        let health = self.ecs.healths.get(self.player)?;
        let needs = self.ecs.needs.get(self.player)?;
        let equipment = self.equipment();
        let rules = &self.rules.survival.rules;
        let options = self.survival_options;
        let survival = !self.is_creative();
        let shown = |on: bool, value: f32| (survival && on).then_some(value.round());
        let max_breath = breath_ticks(rules, equipment.stats.get(WornStat::Breath));
        let max_stamina = Needs::max_stamina(rules, equipment.stats.burden.stamina);
        let effects = self.ecs.effects.get(self.player);
        let effects = effects.map_or_else(Vec::new, |effects| self.effect_views(effects));
        let using = self.ecs.controls.get(self.player).and_then(|c| c.using);
        let using = using.map(|using| {
            let done = self.tick.saturating_sub(using.started);
            let left = u64::from(using.needed).saturating_sub(done);
            let left = u16::try_from(left).unwrap_or(u16::MAX);
            (using.item, left.div_ceil(TICK_STEP) * TICK_STEP)
        });
        Some(VitalsView {
            health: health.current,
            max_health: health.max,
            food: shown(options.hunger, needs.food),
            max_food: rules.hunger.max,
            saturation: needs.saturation.round(),
            thirst: shown(options.thirst, needs.thirst),
            max_thirst: rules.thirst.max,
            stamina: shown(options.stamina, quantized(needs.stamina)),
            max_stamina: max_stamina.round(),
            breath: needs.breath.min(max_breath) / TICK_STEP * TICK_STEP,
            max_breath,
            can_sprint: self
                .ecs
                .controls
                .get(self.player)
                .is_none_or(|c| c.movement.can_sprint),
            effects,
            using,
            temperature: self.temperature_view().filter(|_| survival),
            speed: self.movement_speed(self.player),
        })
    }

    /// The felt temperature in half degrees, its zone, whether still wet.
    fn temperature_view(&self) -> Option<TemperatureView> {
        let rules = &self.rules.survival.temperature;
        let body = self.ecs.temperatures.get(self.player)?;
        let felt = body.felt.filter(|_| self.survival_options.temperature)?;
        Some(TemperatureView {
            felt: (felt * 2.0).round() / 2.0,
            zone: body.zone(rules),
            wet: body.wetness > 0.0,
        })
    }

    fn effect_views(&self, effects: &StatusEffects) -> Vec<EffectView> {
        let defs = &self.rules.survival.effects;
        effects
            .0
            .iter()
            .map(|active| EffectView {
                effect: active.effect.clone(),
                seconds_left: active.ticks_left.div_ceil(TICKS_PER_SECOND),
                harmful: defs.get(&active.effect).is_some_and(|def| def.harmful),
            })
            .collect()
    }

    pub(super) fn send_vitals(&mut self) {
        let view = self.vitals_view();
        if view.is_some() && view != self.vitals_sent {
            self.vitals_sent.clone_from(&view);
            if let Some(view) = view {
                self.messages.push(ServerMessage::Vitals(Box::new(view)));
            }
        }
    }
}

fn quantized(stamina: f32) -> f32 {
    (stamina / STAMINA_STEP).round() * STAMINA_STEP
}
