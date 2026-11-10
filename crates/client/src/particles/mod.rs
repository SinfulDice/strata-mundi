//! Particles: fragments of broken blocks, drips under ceilings, splashes and bubbles.
//! Purely client-side (the server never hears of them), simulated on the main thread in
//! a pool allocated once, drawn in a single instanced call.

mod emit;
mod physics;
mod pool;
mod table;
#[cfg(test)]
mod tests;

pub use pool::{LiveParticle, Look};
pub use table::{ParticleBlock, ParticleTable};

use crate::render::ParticleInstance;
use crate::settings::ParticleMode;
use crate::world::ClientWorld;
use glam::DVec3;
use physics::{Cell, Fate, cell_of};
use pool::{Pool, Rng};
use strata_core::coords::BlockPos;

/// Limits of a particle setting.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Budget {
    capacity: usize,
    /// Divides the bursts of breaking blocks (rounded up).
    burst_divisor: u16,
    /// Cells looked at per second for drips and bubbles.
    samples_per_second: f32,
}

impl Budget {
    fn of(mode: ParticleMode) -> Self {
        match mode {
            ParticleMode::Off => Self {
                capacity: 0,
                burst_divisor: 1,
                samples_per_second: 0.0,
            },
            ParticleMode::Reduced => Self {
                capacity: 1024,
                burst_divisor: 4,
                samples_per_second: 800.0,
            },
            ParticleMode::Full => Self {
                capacity: 4096,
                burst_divisor: 1,
                samples_per_second: 2400.0,
            },
        }
    }
}

/// Adds particles to the pool.
struct Spawner<'a> {
    table: &'a ParticleTable,
    pool: &'a mut Pool,
    rng: &'a mut Rng,
}

pub struct ParticleSystem {
    table: ParticleTable,
    budget: Budget,
    pool: Pool,
    rng: Rng,
    /// Emitter samples owed to the next frames.
    samples_due: f32,
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new(ParticleTable::default(), ParticleMode::Off)
    }
}

impl ParticleSystem {
    pub fn new(table: ParticleTable, mode: ParticleMode) -> Self {
        let budget = Budget::of(mode);
        Self {
            table,
            budget,
            pool: Pool::with_capacity(budget.capacity),
            rng: Rng::new(0x5EED_D209),
            samples_due: 0.0,
        }
    }

    /// The pool is allocated again (emptied) only when its size changes.
    pub fn set_mode(&mut self, mode: ParticleMode) {
        let budget = Budget::of(mode);
        if budget.capacity != self.budget.capacity {
            self.pool = Pool::with_capacity(budget.capacity);
        }
        self.budget = budget;
    }

    /// New content (resources reloaded): live particles may name old ids.
    pub fn set_table(&mut self, table: ParticleTable) {
        self.table = table;
        self.pool.clear();
    }

    pub fn len(&self) -> usize {
        self.pool.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pool.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.pool.capacity()
    }

    /// Emits around `camera`, then moves every particle by `dt` seconds.
    pub fn update(&mut self, dt: f32, camera: DVec3, world: &ClientWorld) {
        if self.budget.capacity == 0 {
            return;
        }
        self.sample_emitters(dt, camera, world);
        let (table, pool, rng) = (&self.table, &mut self.pool, &mut self.rng);
        let cell = |pos: BlockPos| {
            let block = table.block(world.block_at(pos));
            Cell {
                solid: block.solid,
                liquid: block.liquid,
            }
        };
        let mut spawner = Spawner { table, pool, rng };
        let mut index = 0;
        while let Some(particle) = spawner.pool.get_mut(index) {
            let fate = match table.particle(particle.kind) {
                Some(def) => physics::step(particle, def, dt, &cell),
                None => Fate::Dead,
            };
            match fate {
                Fate::Alive => index += 1,
                Fate::Dead => {
                    spawner.pool.remove(index);
                }
                Fate::Impact => {
                    let ended = spawner.pool.remove(index);
                    spawner.splash(&ended);
                }
            }
        }
    }

    /// One square per particle, around the camera, lit by its cell (`out` keeps its
    /// memory from frame to frame).
    pub fn write_instances(
        &self,
        camera: DVec3,
        world: &ClientWorld,
        out: &mut Vec<ParticleInstance>,
    ) {
        out.clear();
        for particle in self.pool.iter() {
            let Some(def) = self.table.particle(particle.kind) else {
                continue;
            };
            let remaining = (particle.lifetime - particle.age) / (0.25 * particle.lifetime);
            let size = particle.size * remaining.clamp(0.0, 1.0);
            let centre = particle.position + DVec3::Y * f64::from(size * 0.5);
            let light = world.light_at(cell_of(centre));
            let block_light = if def.glows {
                1.0
            } else {
                f32::from(light.block()) / 15.0
            };
            let Look { layer, uv, color } = particle.look;
            out.push(ParticleInstance {
                position: (centre - camera).as_vec3().to_array(),
                size,
                color,
                layer,
                uv,
                light: [f32::from(light.sky()) / 15.0, block_light],
            });
        }
    }
}
