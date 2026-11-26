//! Temperature of every body that has one: wetness each tick, the surroundings sampled
//! every `sample_every` ticks (biome, height, time of day, roof, water, the strongest heat
//! source within reach, clothing), the felt temperature following, and the harm of the
//! extreme zones. Off with the world option, nothing in creative.

use super::Game;
use crate::ecs::Entity;
use crate::loader::ColumnLoader;
use glam::DVec3;
use std::sync::Arc;
use strata_core::coords::BlockPos;
use strata_core::survival::temperature::{
    BodyTemperature, Insulation, Strain, Surroundings, contributions, total,
};
use strata_core::survival::{Defences, Survival, resistance};

impl Game {
    /// 0 at night, 1 by day: the server's clock tells it each tick.
    pub fn set_daylight(&mut self, daylight: f32) {
        self.daylight = daylight.clamp(0.0, 1.0);
    }

    /// What the temperature of `entity` does to it now (nothing when off or creative).
    pub(super) fn temperature_strain(&self, entity: Entity) -> Strain {
        let rules = &self.rules.survival.temperature;
        let felt = self.ecs.temperatures.get(entity);
        match felt {
            Some(body) if self.survival_options.temperature && !self.is_creative_entity(entity) => {
                rules.strain(body.zone(rules))
            }
            _ => Strain::NONE,
        }
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
        let due = state.felt.is_none() || self.tick.is_multiple_of(u64::from(rules.sample_every));
        let sample = due.then(|| self.sample(entity, immersed, loader, &survival));
        let Some(state) = self.ecs.temperatures.get_mut(entity) else {
            return;
        };
        state.soak(wet, state.heat > 0.0, rules);
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
        &self,
        entity: Entity,
        immersed: bool,
        loader: &ColumnLoader,
        survival: &Survival,
    ) -> (f32, f32) {
        let rules = &survival.temperature;
        let Some(body) = self.ecs.bodies.get(entity) else {
            return (rules.comfort(), 0.0);
        };
        let feet = cell(body.position);
        let biome = loader
            .biome(feet)
            .and_then(|b| self.biomes.get(usize::from(b.0)));
        let aabb = body.aabb();
        let heat = self.heat_near((aabb.min + aabb.max) * 0.5, loader);
        let around = Surroundings {
            biome: biome.cloned(),
            dimension: self.dimension.clone(),
            height: body.position.y,
            daylight: self.daylight,
            sheltered: self.sheltered(cell(body.eyes()), rules.shelter.height, loader),
            immersed,
            heat,
            extra: Vec::new(),
        };
        let wetness = self.ecs.temperatures.get(entity).map_or(0.0, |t| t.wetness);
        let parts = contributions(rules, &around, wetness, self.clothing(entity));
        (total(&parts), heat)
    }

    /// The warmth of the strongest heat source within reach of `centre`.
    fn heat_near(&self, centre: DVec3, loader: &ColumnLoader) -> f32 {
        let heat = &self.rules.survival.heat;
        let reach = i32::from(heat.reach);
        if reach == 0 {
            return 0.0;
        }
        let middle = cell(centre);
        let mut strongest = 0.0f32;
        for x in -reach..=reach {
            for z in -reach..=reach {
                for y in -reach..=reach {
                    let pos = BlockPos::new(middle.x + x, middle.y + y, middle.z + z);
                    let Some(source) = loader.block(pos).and_then(|b| heat.get(b)) else {
                        continue;
                    };
                    let distance = (block_centre(pos) - centre).length() as f32;
                    strongest = strongest.max(source.at((distance - 0.5).max(0.0)));
                }
            }
        }
        strongest
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
        Insulation {
            cold: resistance(&clothing.cold_stat, &defences, &survival.effects),
            heat: resistance(&clothing.heat_stat, &defences, &survival.effects),
        }
    }

    /// A body starts at the temperature of where it stands (sampled at its first tick).
    pub(super) fn fresh_temperature(&mut self, entity: Entity) {
        self.ecs
            .temperatures
            .insert(entity, BodyTemperature::default());
    }
}

fn cell(point: DVec3) -> BlockPos {
    BlockPos::new(
        point.x.floor() as i32,
        point.y.floor() as i32,
        point.z.floor() as i32,
    )
}

fn block_centre(pos: BlockPos) -> DVec3 {
    DVec3::new(
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    )
}
