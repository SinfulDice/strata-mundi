//! The survival gauges each tick: effort and time wear the needs down, the needs heal or
//! hurt, effects pulse, and the world around hurts (falls, drowning, lava, fire, a head
//! in a block). Creative: nothing goes down and nothing hurts.

use super::Game;
use super::terrain::WorldTerrain;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::coords::BlockPos;
use strata_core::physics::{Body, Terrain};
use strata_core::survival::{
    Contact, EffectPulse, Exertion, Health, NeedHarm, Needs, NeedsContext, StatusEffects,
    SurvivalOptions, breath_ticks, engine, fall_damage,
};
use strata_core::worn::WornStat;
use strata_protocol::{EffectView, ServerMessage, VitalsView};

/// What the body is in this tick.
#[derive(Debug, Default)]
struct Surroundings {
    /// The eyes are in water (or another liquid that does not burn).
    underwater: bool,
    /// Some of the body is in such a liquid (puts out burning).
    wet: bool,
    /// The eyes are inside a solid block.
    buried: bool,
    /// What the blocks touched do, the strongest of each kind of damage.
    contacts: Vec<Contact>,
}

impl Game {
    /// World options: which needs count and how fast.
    pub fn set_survival_options(&mut self, options: SurvivalOptions) {
        self.survival_options = options;
    }

    /// Where the player wakes after fainting.
    pub fn set_spawn(&mut self, spawn: DVec3) {
        self.spawn = spawn;
    }

    /// Gives the player full gauges (a new character, or one from an older world).
    pub(super) fn fresh_vitals(&mut self) {
        let rules = &self.rules.survival.rules;
        let max = rules.health.max;
        let needs = Needs::full(rules);
        self.ecs.healths.insert(self.player, Health::full(max));
        self.ecs.needs.insert(self.player, needs);
        self.ecs
            .effects
            .insert(self.player, StatusEffects::default());
    }

    pub(super) fn vitals_tick(&mut self, loader: &ColumnLoader) {
        let creative = self.is_creative();
        let (effort, fallen) = self.take_effort();
        if !creative {
            let around = self.surroundings(loader);
            self.needs_tick(&around, &effort);
            self.effects_tick(around.wet);
            self.hazards_tick(&around, fallen);
            self.feed_from_bags();
            if self
                .ecs
                .healths
                .get(self.player)
                .is_some_and(Health::is_dead)
            {
                self.faint(loader);
            }
        }
        self.use_tick();
        self.refresh_sprint();
        self.send_vitals();
    }

    /// The effort counted since the last tick, and the last fall, spent now.
    fn take_effort(&mut self) -> (Exertion, f64) {
        self.ecs
            .controls
            .get_mut(self.player)
            .map_or((Exertion::default(), 0.0), |control| {
                let effort = std::mem::take(&mut control.exertion);
                (effort, std::mem::take(&mut control.fallen))
            })
    }

    fn needs_tick(&mut self, around: &Surroundings, effort: &Exertion) {
        let equipment = self.equipment();
        let rules = &self.rules.survival.rules;
        let max_health = rules.health.max + equipment.stats.get(WornStat::MaxHealth);
        let context = NeedsContext {
            options: self.survival_options,
            underwater: around.underwater,
            extra_breath: equipment.stats.get(WornStat::Breath),
            stamina_burden: equipment.stats.burden.stamina,
        };
        let (Some(needs), Some(health)) = (
            self.ecs.needs.get_mut(self.player),
            self.ecs.healths.get_mut(self.player),
        ) else {
            return;
        };
        health.set_max(max_health);
        health.tick();
        needs.exert(effort, rules, &context);
        let harm = needs.tick(health, rules, &context);
        for (kind, amount) in harm {
            let kind = match kind {
                NeedHarm::Starvation => "starvation",
                NeedHarm::Dehydration => "dehydration",
                NeedHarm::Drowning => "drowning",
            };
            self.hurt(&engine(kind), amount);
        }
    }

    fn effects_tick(&mut self, wet: bool) {
        let defs = &self.rules.survival.effects;
        let Some(effects) = self.ecs.effects.get_mut(self.player) else {
            return;
        };
        for pulse in effects.tick(defs, wet) {
            match pulse {
                EffectPulse::Heal(amount) => {
                    if let Some(health) = self.ecs.healths.get_mut(self.player) {
                        health.heal(amount);
                    }
                }
                EffectPulse::Damage { kind, amount } => {
                    self.hurt(&kind, amount);
                }
            }
        }
    }

    fn hazards_tick(&mut self, around: &Surroundings, fallen: f64) {
        let rules = &self.rules.survival.rules;
        let fall = fall_damage(fallen, &rules.fall);
        let suffocation = rules.suffocation_damage;
        let every = u64::from(rules.suffocation_every);
        if fall > 0.0 {
            self.hurt(&engine("fall"), fall);
        }
        if around.buried && self.tick.is_multiple_of(every) {
            self.hurt(&engine("suffocation"), suffocation);
        }
        for contact in &around.contacts {
            self.hurt(&contact.damage, contact.amount);
            if let (Some(grant), Some(effects)) =
                (&contact.effect, self.ecs.effects.get_mut(self.player))
            {
                effects.give(grant);
            }
        }
    }

    /// Water at the eyes and around the body, a block at the eyes, the blocks touched.
    fn surroundings(&self, loader: &ColumnLoader) -> Surroundings {
        let Some(body) = self.player_body().copied() else {
            return Surroundings::default();
        };
        let terrain = WorldTerrain {
            loader,
            rules: &self.rules,
            border: self.border,
        };
        let survival = &self.rules.survival;
        let calm = |pos: BlockPos| {
            let block = loader.block(pos)?;
            let cell = terrain.liquid(pos.x, pos.y, pos.z)?;
            survival.contact(block).is_none().then_some(cell)
        };
        let eyes = body.eyes();
        let eye_cell = cell_of(eyes);
        let underwater =
            calm(eye_cell).is_some_and(|cell| eyes.y < f64::from(eye_cell.y) + cell.surface);
        let mut around = Surroundings {
            underwater,
            buried: terrain.solid(eye_cell.x, eye_cell.y, eye_cell.z)
                && loader.block(eye_cell).is_some(),
            ..Surroundings::default()
        };
        for pos in touched_cells(&body) {
            around.wet |= calm(pos).is_some();
            let contact = loader.block(pos).and_then(|block| survival.contact(block));
            if let Some(contact) = contact {
                strongest(&mut around.contacts, contact);
            }
        }
        around
    }

    /// Sprinting needs stamina and food; the client's prediction is told with the gauges.
    fn refresh_sprint(&mut self) {
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
        Some(VitalsView {
            health: health.current,
            max_health: health.max,
            food: shown(options.hunger, needs.food),
            max_food: rules.hunger.max,
            saturation: needs.saturation.round(),
            thirst: shown(options.thirst, needs.thirst),
            max_thirst: rules.thirst.max,
            stamina: shown(options.stamina, needs.stamina),
            max_stamina: max_stamina.round(),
            breath: needs.breath.min(max_breath) / 10 * 10,
            max_breath,
            can_sprint: self
                .ecs
                .controls
                .get(self.player)
                .is_none_or(|c| c.movement.can_sprint),
            effects,
        })
    }

    fn effect_views(&self, effects: &StatusEffects) -> Vec<EffectView> {
        let defs = &self.rules.survival.effects;
        effects
            .0
            .iter()
            .map(|active| EffectView {
                effect: active.effect.clone(),
                seconds_left: active.ticks_left.div_ceil(20),
                harmful: defs.get(&active.effect).is_some_and(|def| def.harmful),
            })
            .collect()
    }

    fn send_vitals(&mut self) {
        let view = self.vitals_view();
        if view.is_some() && view != self.vitals_sent {
            self.vitals_sent.clone_from(&view);
            if let Some(view) = view {
                self.messages.push(ServerMessage::Vitals(Box::new(view)));
            }
        }
    }
}

fn cell_of(point: DVec3) -> BlockPos {
    BlockPos::new(
        point.x.floor() as i32,
        point.y.floor() as i32,
        point.z.floor() as i32,
    )
}

/// The cells the body's box overlaps, a little inside its faces (standing on lava does
/// not count as being in it, the block under the feet aside).
fn touched_cells(body: &Body) -> impl Iterator<Item = BlockPos> {
    let aabb = body.aabb();
    let (min, max) = (cell_of(aabb.min + 0.001), cell_of(aabb.max - 0.001));
    (min.x..=max.x).flat_map(move |x| {
        (min.y..=max.y).flat_map(move |y| (min.z..=max.z).map(move |z| BlockPos::new(x, y, z)))
    })
}

/// Keeps one contact per kind of damage: the strongest.
fn strongest(contacts: &mut Vec<Contact>, contact: &Contact) {
    match contacts.iter_mut().find(|c| c.damage == contact.damage) {
        Some(known) if known.amount < contact.amount => *known = contact.clone(),
        Some(_) => {}
        None => contacts.push(contact.clone()),
    }
}

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
    }
}
