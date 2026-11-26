//! Temperature of every body that has one: wetness each tick, the surroundings sampled
//! every `sample_every` ticks, each body on its own tick (biome, height, time of day, roof,
//! water, the strongest heat source within reach, clothing), the felt temperature
//! following, and the harm of the extreme zones. Off with the world option, nothing in
//! creative. The heat found is kept while the body stays in its cell and no block changes
//! within reach.

use super::Game;
use crate::ecs::{Entity, HeatCache};
use crate::loader::ColumnLoader;
use glam::DVec3;
use std::sync::Arc;
use strata_core::coords::{BlockPos, ColumnPos};
use strata_core::survival::temperature::{
    Insulation, Strain, Surroundings, strongest_heat, target,
};
use strata_core::survival::{Defences, Survival, resistance};
use strata_core::updates::BlockChange;

impl Game {
    /// 0 at night, 1 by day: the server's clock tells it each tick.
    pub fn set_daylight(&mut self, daylight: f32) {
        self.daylight = daylight.clamp(0.0, 1.0);
    }

    /// What the temperature of `entity` does to it now (nothing when off or creative).
    pub(super) fn temperature_strain(&self, entity: Entity) -> Strain {
        let rules = &self.rules.survival.temperature;
        match self.ecs.temperatures.get(entity) {
            Some(body) if self.survival_options.temperature && !self.is_creative_entity(entity) => {
                rules.strain(body.zone(rules))
            }
            _ => Strain::NONE,
        }
    }

    /// The share of its walking speed `entity` keeps: heavy armour and the cold.
    pub(super) fn movement_speed(&self, entity: Entity) -> f32 {
        let burden = self
            .worn_by(entity)
            .map_or(1.0, |e| e.stats.burden.speed_factor());
        burden * self.temperature_strain(entity).speed
    }

    /// One tick of temperature for `entity`: `wet` when some of it is in water,
    /// `immersed` with the head under.
    pub(super) fn temperature_tick(
        &mut self,
        entity: Entity,
        (wet, immersed): (bool, bool),
        loader: &ColumnLoader,
    ) {
        if !self.survival_options.temperature {
            return;
        }
        let survival = Arc::clone(&self.rules.survival);
        let rules = &survival.temperature;
        let Some(state) = self.ecs.temperatures.get(entity) else {
            return;
        };
        let every = u64::from(rules.sample_every);
        let due = state.felt.is_none() || (self.tick + entity.index() as u64).is_multiple_of(every);
        let sample = due.then(|| self.sample(entity, immersed, loader, &survival));
        let Some(state) = self.ecs.temperatures.get_mut(entity) else {
            return;
        };
        state.soak(wet, rules);
        if let Some((target, heat)) = sample {
            state.heat = heat;
            state.follow(target, rules.sample_every, rules);
        }
        if let Some((kind, amount)) = state.harm_tick(rules) {
            self.hurt(entity, &kind, amount);
        }
    }

    /// The temperature the surroundings of `entity` come to, and the heat nearby.
    fn sample(
        &mut self,
        entity: Entity,
        immersed: bool,
        loader: &ColumnLoader,
        survival: &Survival,
    ) -> (f32, f32) {
        let rules = &survival.temperature;
        let Some(body) = self.ecs.bodies.get(entity).copied() else {
            return (rules.comfort(), 0.0);
        };
        let aabb = body.aabb();
        let heat = self.heat_near(entity, (aabb.min + aabb.max) * 0.5, loader);
        let biome = loader.biome(cell(body.position));
        let around = Surroundings {
            biome: biome.and_then(|b| self.biomes.get(usize::from(b.0))),
            dimension: self.dimension.as_ref(),
            height: body.position.y,
            daylight: self.daylight,
            sheltered: self.sheltered(cell(body.eyes()), rules.shelter.height, loader),
            immersed,
            heat,
            extra: &[],
        };
        let wetness = self.ecs.temperatures.get(entity).map_or(0.0, |t| t.wetness);
        (target(rules, &around, wetness, self.clothing(entity)), heat)
    }

    /// The strongest heat at `centre`, kept for `entity` while it stays in the same cell.
    fn heat_near(&mut self, entity: Entity, centre: DVec3, loader: &ColumnLoader) -> f32 {
        let here = cell(centre);
        if let Some(cache) = self.ecs.heat_caches.get(entity)
            && cache.cell == here
        {
            return cache.warmth;
        }
        let survival = &self.rules.survival;
        let found = strongest_heat(
            &survival.heat,
            survival.temperature.walls,
            centre,
            |pos: ColumnPos| loader.column(pos),
            |pos| loader.block(pos).is_some_and(|b| self.rules.is_solid(b)),
        );
        let warmth = found.map_or(0.0, |f| f.warmth);
        self.ecs
            .heat_caches
            .insert(entity, HeatCache { cell: here, warmth });
        warmth
    }

    /// Forgets the heat found near blocks that changed (a fire lit, a furnace gone out),
    /// or everywhere when columns arrived.
    pub(super) fn forget_heat(&mut self, changes: &[BlockChange], columns_arrived: bool) {
        let reach = i32::from(self.rules.survival.heat.reach) + 1;
        let stale: Vec<Entity> = (self.ecs.heat_caches.iter())
            .filter(|(_, cache)| {
                columns_arrived
                    || changes.iter().any(|(pos, _)| {
                        (pos.x - cache.cell.x).abs() <= reach
                            && (pos.y - cache.cell.y).abs() <= reach
                            && (pos.z - cache.cell.z).abs() <= reach
                    })
            })
            .map(|(entity, _)| entity)
            .collect();
        for entity in stale {
            self.ecs.heat_caches.remove(entity);
        }
    }

    /// A solid block over the head, up to `height` blocks above.
    fn sheltered(&self, eyes: BlockPos, height: u8, loader: &ColumnLoader) -> bool {
        (1..=i32::from(height)).any(|dy| {
            let pos = BlockPos::new(eyes.x, eyes.y + dy, eyes.z);
            loader
                .block(pos)
                .is_some_and(|block| self.rules.is_solid(block))
        })
    }

    /// Clothing stats of what `entity` wears, or what an effect gives (the stronger).
    fn clothing(&self, entity: Entity) -> Insulation {
        let survival = &self.rules.survival;
        let clothing = &survival.temperature.clothing;
        let equipment = self.worn_by(entity);
        let defences = Defences {
            protection: 0,
            worn: equipment.as_ref().map(|e| &e.stats.values),
            effects: self.ecs.effects.get(entity),
        };
        let points = |stat| {
            let worn = (defences.worn.and_then(|worn| worn.get(stat))).copied();
            let worn = worn.unwrap_or(0.0);
            // Negative clothing (a curse) counts even without an effect; else the stronger.
            if worn < 0.0
                && !defences
                    .effects
                    .is_some_and(|e| e.stat(&survival.effects, stat) > 0.0)
            {
                worn
            } else {
                resistance(stat, &defences, &survival.effects)
            }
        };
        Insulation {
            cold: points(&clothing.cold_stat),
            heat: points(&clothing.heat_stat),
        }
    }

    /// A body starts at the temperature of where it stands (sampled at its first tick).
    pub(super) fn fresh_temperature(&mut self, entity: Entity) {
        self.ecs.temperatures.insert(entity, Default::default());
        self.ecs.heat_caches.remove(entity);
    }
}

fn cell(point: DVec3) -> BlockPos {
    BlockPos::new(
        point.x.floor() as i32,
        point.y.floor() as i32,
        point.z.floor() as i32,
    )
}
