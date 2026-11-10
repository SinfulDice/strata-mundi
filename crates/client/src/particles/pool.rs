//! A fixed number of live particles, allocated once: a full pool drops new particles.

use glam::{DVec3, Vec3};
use strata_core::content::ParticleId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveParticle {
    pub kind: ParticleId,
    /// Bottom centre of the square, in the world.
    pub position: DVec3,
    /// Blocks per second.
    pub velocity: Vec3,
    pub age: f32,
    pub lifetime: f32,
    pub size: f32,
    pub look: Look,
    pub on_ground: bool,
}

/// What a particle shows: a piece of a texture layer, multiplied by a colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub layer: u32,
    /// Corner of the piece and its edge, in texture coordinates (0 to 1).
    pub uv: [f32; 3],
    /// Linear colour factor.
    pub color: [f32; 3],
}

#[derive(Debug)]
pub struct Pool {
    live: Vec<LiveParticle>,
}

impl Pool {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            live: Vec::with_capacity(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.live.capacity()
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_full(&self) -> bool {
        self.live.len() >= self.live.capacity()
    }

    /// `false` (the particle is dropped) when the pool is full: it never grows.
    pub fn push(&mut self, particle: LiveParticle) -> bool {
        if self.is_full() {
            return false;
        }
        self.live.push(particle);
        true
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut LiveParticle> {
        self.live.get_mut(index)
    }

    /// Removes particle `index`, the last one taking its place.
    pub fn remove(&mut self, index: usize) -> LiveParticle {
        self.live.swap_remove(index)
    }

    pub fn clear(&mut self) {
        self.live.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &LiveParticle> {
        self.live.iter()
    }
}

/// SplitMix64: particles only need to look random, not to be reproducible across runs.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn between(&mut self, [low, high]: [f32; 2]) -> f32 {
        low + (high - low) * self.unit()
    }

    /// Uniform in `-reach..=reach`.
    pub fn offset(&mut self, reach: i32) -> i32 {
        let span = (2 * reach + 1).max(1) as u64;
        (self.next_u64() % span) as i32 - reach
    }

    /// A direction, slightly favouring up so bursts spread like a small fountain.
    pub fn direction(&mut self) -> Vec3 {
        let v = Vec3::new(
            self.unit() * 2.0 - 1.0,
            self.unit() * 1.6 - 0.4,
            self.unit() * 2.0 - 1.0,
        );
        v.try_normalize().unwrap_or(Vec3::Y)
    }
}
