//! Movement of one particle: gravity, drag and stops on full blocks, axis by axis.

use super::pool::LiveParticle;
use glam::DVec3;
use strata_core::content::Particle;
use strata_core::coords::BlockPos;

/// What a cell does to particles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cell {
    pub solid: bool,
    pub liquid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fate {
    Alive,
    Dead,
    /// Landed or fell into a liquid: it ends, throwing its impact particles.
    Impact,
}

/// Longest move between two collision checks, so fast particles never cross a block.
const MAX_STEP: f32 = 0.4;
const MAX_SUBSTEPS: u32 = 8;
/// Fraction of the sliding speed lost per second on the ground.
const GROUND_FRICTION: f32 = 8.0;
const GAP: f64 = 1e-3;

pub fn cell_of(position: DVec3) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

pub fn step(
    particle: &mut LiveParticle,
    def: &Particle,
    dt: f32,
    cell: &impl Fn(BlockPos) -> Cell,
) -> Fate {
    particle.age += dt;
    if particle.age >= particle.lifetime {
        return Fate::Dead;
    }
    if def.in_liquid && !cell(cell_of(particle.position)).liquid {
        return Fate::Dead;
    }
    let moving = (particle.age - def.hang).min(dt);
    if moving <= 0.0 {
        return Fate::Alive;
    }
    accelerate(particle, def, moving);
    let landed = travel(particle, def, moving, cell);
    if def.impact.is_some() {
        let in_liquid = !def.in_liquid && cell(cell_of(particle.position)).liquid;
        if landed || in_liquid {
            return Fate::Impact;
        }
    }
    Fate::Alive
}

fn accelerate(particle: &mut LiveParticle, def: &Particle, dt: f32) {
    let velocity = &mut particle.velocity;
    velocity.y -= def.gravity * dt;
    *velocity *= def.drag.powf(dt);
    if particle.on_ground {
        let keep = (1.0 - GROUND_FRICTION * dt).max(0.0);
        velocity.x *= keep;
        velocity.z *= keep;
    }
}

/// Moves by the particle's velocity; `true` when it came down on a full block.
fn travel(
    particle: &mut LiveParticle,
    def: &Particle,
    dt: f32,
    cell: &impl Fn(BlockPos) -> Cell,
) -> bool {
    let distance = (particle.velocity * dt).abs().max_element();
    let steps = ((distance / MAX_STEP).ceil() as u32).clamp(1, MAX_SUBSTEPS);
    let sub = dt / steps as f32;
    let mut landed = false;
    for _ in 0..steps {
        let delta = (particle.velocity * sub).as_dvec3();
        for axis in [1, 0, 2] {
            landed |= move_axis(particle, axis, delta[axis], def.collides, cell);
        }
    }
    particle.on_ground =
        def.collides && cell(cell_of(particle.position - DVec3::Y * GAP * 2.0)).solid;
    landed
}

fn move_axis(
    particle: &mut LiveParticle,
    axis: usize,
    delta: f64,
    collides: bool,
    cell: &impl Fn(BlockPos) -> Cell,
) -> bool {
    if delta == 0.0 {
        return false;
    }
    let mut moved = particle.position;
    moved[axis] += delta;
    if !collides || !cell(cell_of(moved)).solid {
        particle.position = moved;
        return false;
    }
    particle.velocity[axis] = 0.0;
    if axis != 1 {
        return false;
    }
    let block = moved.y.floor();
    particle.position.y = if delta < 0.0 {
        block + 1.0
    } else {
        block - GAP
    };
    delta < 0.0
}
