//! What the ears can tell of the place: walls between a sound and the listener, ground
//! overhead, biomes around. Read through a small trait so tests build scenes by hand.

use super::triggers::{SoundTable, cell};
use crate::world::{ClientWorld, column_of};
use glam::DVec3;
use strata_core::chunk::BiomeId;
use strata_core::coords::{BlockPos, SECTION_SIZE};

pub trait Surroundings {
    /// How much the block muffles sound through it, 0 to 1.
    fn density(&self, pos: BlockPos) -> f32;
    fn sky_light(&self, pos: BlockPos) -> u8;
    fn biome(&self, x: i32, z: i32) -> Option<BiomeId>;
    /// Height of the highest full block of the column; `None` if not loaded.
    fn surface(&self, x: i32, z: i32) -> Option<i32>;
}

/// The client's world as heard.
pub struct WorldView<'a> {
    pub world: &'a ClientWorld,
    pub table: &'a SoundTable,
}

/// Blocks at least this dense stop a probing ray and make the ground.
pub const WALL: f32 = 0.5;

impl Surroundings for WorldView<'_> {
    fn density(&self, pos: BlockPos) -> f32 {
        self.table.get(self.world.block_at(pos)).density
    }

    fn sky_light(&self, pos: BlockPos) -> u8 {
        self.world.light_at(pos).sky()
    }

    fn biome(&self, x: i32, z: i32) -> Option<BiomeId> {
        self.world.biome_at(x, z)
    }

    fn surface(&self, x: i32, z: i32) -> Option<i32> {
        let top = self.world.top_section(column_of(x, z))?;
        let highest = (top as i32 + 1) * SECTION_SIZE - 1;
        (0..=highest)
            .rev()
            .find(|&y| self.density(BlockPos::new(x, y, z)) >= 1.0)
            .or(Some(0))
    }
}

/// Points tried along a sound's path (half a block apart, so 32 blocks at most).
const MUFFLE_SAMPLES: usize = 64;
/// Closer than this, nothing stands between a sound and the ears.
const MUFFLE_NEAR: f64 = 1.5;
/// Muffling gained per full block crossed.
const MUFFLE_PER_BLOCK: f32 = 0.8;

/// How muffled a sound at `from` is at `to`: 0 in open air, towards 1 behind thick
/// walls. The cells of the source and of the ears do not count.
pub fn muffling(world: &impl Surroundings, from: DVec3, to: DVec3) -> f32 {
    let offset = to - from;
    let length = offset.length();
    if length < MUFFLE_NEAR {
        return 0.0;
    }
    let steps = ((length * 2.0).ceil() as usize).clamp(2, MUFFLE_SAMPLES);
    let (start, end) = (cell(from), cell(to));
    let mut last = start;
    let mut blocks = 0.0;
    for step in 1..steps {
        let here = cell(from + offset * (step as f64 / steps as f64));
        if here == last || here == end {
            continue;
        }
        last = here;
        blocks += world.density(here);
    }
    1.0 - (-MUFFLE_PER_BLOCK * blocks).exp()
}

/// How far underground the ears are, 0 (outside, or in a house) to 1 (in the rock):
/// little sky light reaches them and there is ground well above.
pub fn underground(world: &impl Surroundings, head: DVec3) -> f32 {
    let here = cell(head);
    let sky = f32::from(world.sky_light(here));
    let enclosed = ((12.0 - sky) / 6.0).clamp(0.0, 1.0);
    let depth = world
        .surface(here.x, here.z)
        .map_or(0.0, |surface| (surface - here.y) as f32);
    let deep = ((depth - 2.0) / 6.0).clamp(0.0, 1.0);
    enclosed * deep
}

/// Spacing and reach of the biome samples around the ears: a biome close by is heard
/// a little (the sea from the beach).
const BIOME_STEP: i32 = 8;
const BIOME_REACH: i32 = 2;

/// The biomes found around `head`, each with its share of the samples.
pub fn biomes_around(world: &impl Surroundings, head: DVec3) -> Vec<(BiomeId, f32)> {
    let here = cell(head);
    let mut found: Vec<(BiomeId, f32)> = Vec::new();
    let side = 2 * BIOME_REACH + 1;
    let share = 1.0 / (side * side) as f32;
    for dx in -BIOME_REACH..=BIOME_REACH {
        for dz in -BIOME_REACH..=BIOME_REACH {
            let Some(biome) = world.biome(here.x + dx * BIOME_STEP, here.z + dz * BIOME_STEP)
            else {
                continue;
            };
            match found.iter_mut().find(|(known, _)| *known == biome) {
                Some((_, weight)) => *weight += share,
                None => found.push((biome, share)),
            }
        }
    }
    found
}
