//! The survival gauges each tick: effort and time wear the needs down, the needs heal or
//! hurt, effects pulse, and the world around hurts (falls, drowning, lava, fire, a head
//! in a block). Creative: nothing goes down and nothing hurts.

use super::Game;
use super::terrain::WorldTerrain;
use crate::ecs::Entity;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::coords::BlockPos;
use strata_core::physics::{Body, Terrain};
use strata_core::survival::{
    Contact, EffectPulse, Exertion, Health, NeedHarm, Needs, NeedsContext, StatusEffects,
    SurvivalOptions, engine, fall_damage,
};
use strata_core::worn::WornStat;

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
        let living: Vec<Entity> = self.ecs.healths.iter().map(|(e, _)| e).collect();
        for entity in living {
            let (effort, fallen) = self.take_effort(entity);
            if self.is_creative_entity(entity) {
                continue;
            }
            let around = self.surroundings(entity, loader);
            self.needs_tick(entity, &around, &effort);
            self.effects_tick(entity, around.wet);
            self.hazards_tick(entity, &around, fallen);
        }
        if !self.is_creative() {
            self.feed_from_bags();
        }
        if self
            .ecs
            .healths
            .get(self.player)
            .is_some_and(Health::is_dead)
        {
            self.faint(loader);
        }
        self.use_tick();
        self.refresh_sprint();
        self.send_vitals();
    }

    /// The effort counted since the last tick, and the last fall, spent now.
    fn take_effort(&mut self, entity: Entity) -> (Exertion, f64) {
        self.ecs
            .controls
            .get_mut(entity)
            .map_or((Exertion::default(), 0.0), |control| {
                let effort = std::mem::take(&mut control.exertion);
                (effort, std::mem::take(&mut control.fallen))
            })
    }

    /// Health follows what is worn; a body with needs (a player) wears them down.
    fn needs_tick(&mut self, entity: Entity, around: &Surroundings, effort: &Exertion) {
        let equipment = self.worn_by(entity);
        let worn = |stat| equipment.as_ref().map_or(0.0, |e| e.stats.get(stat));
        let rules = &self.rules.survival.rules;
        let context = NeedsContext {
            options: self.survival_options,
            underwater: around.underwater,
            extra_breath: worn(WornStat::Breath),
            stamina_burden: equipment.as_ref().map_or(0.0, |e| e.stats.burden.stamina),
        };
        let Some(health) = self.ecs.healths.get_mut(entity) else {
            return;
        };
        health.set_max(rules.health.max + worn(WornStat::MaxHealth));
        health.tick();
        let Some(needs) = self.ecs.needs.get_mut(entity) else {
            return;
        };
        needs.exert(effort, rules, &context);
        let harm = needs.tick(health, rules, &context);
        for (kind, amount) in harm {
            let kind = match kind {
                NeedHarm::Starvation => "starvation",
                NeedHarm::Dehydration => "dehydration",
                NeedHarm::Drowning => "drowning",
            };
            self.hurt(entity, &engine(kind), amount);
        }
    }

    fn effects_tick(&mut self, entity: Entity, wet: bool) {
        let defs = &self.rules.survival.effects;
        let Some(effects) = self.ecs.effects.get_mut(entity) else {
            return;
        };
        for pulse in effects.tick(defs, wet) {
            match pulse {
                EffectPulse::Heal(amount) => {
                    if let Some(health) = self.ecs.healths.get_mut(entity) {
                        health.heal(amount);
                    }
                }
                EffectPulse::Damage { kind, amount } => {
                    self.hurt(entity, &kind, amount);
                }
            }
        }
    }

    /// Falls, a head in a block, and the blocks touched, each at its own pace.
    fn hazards_tick(&mut self, entity: Entity, around: &Surroundings, fallen: f64) {
        let rules = &self.rules.survival.rules;
        let fall = fall_damage(fallen, &rules.fall);
        let suffocation = rules.suffocation_damage;
        let every = u64::from(rules.suffocation_every);
        if fall > 0.0 {
            self.hurt(entity, &engine("fall"), fall);
        }
        if around.buried && self.tick.is_multiple_of(every) {
            self.hurt(entity, &engine("suffocation"), suffocation);
        }
        for contact in &around.contacts {
            if !self.tick.is_multiple_of(u64::from(contact.every.max(1))) {
                continue;
            }
            self.hurt(entity, &contact.damage, contact.amount);
            if let (Some(grant), Some(effects)) =
                (&contact.effect, self.ecs.effects.get_mut(entity))
            {
                effects.give(grant);
            }
        }
    }

    /// Water at the eyes and around the body, a block at the eyes, the blocks touched.
    fn surroundings(&self, entity: Entity, loader: &ColumnLoader) -> Surroundings {
        let Some(body) = self.ecs.bodies.get(entity).copied() else {
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
        Some(known)
            if known.amount * f32::from(contact.every)
                < contact.amount * f32::from(known.every) =>
        {
            *known = contact.clone();
        }
        Some(_) => {}
        None => contacts.push(contact.clone()),
    }
}
