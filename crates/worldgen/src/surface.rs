//! Blocks of the ground: fills one block column top-down from the 3D terrain, with the
//! biome's surface, soil, peaks, bare rock on steep slopes and the sea.

use crate::Seed;
use crate::climate::Sample;
use crate::content::{Bands, Biome, WorldgenContent};
use crate::noise::Noise;
use crate::terrain::{Grid, is_solid};
use strata_core::chunk::Column;
use strata_core::content::BlockId;

/// What decides the blocks of one block column.
pub struct Ground<'a> {
    pub sample: &'a Sample,
    pub biome: &'a Biome,
    pub steep: bool,
    /// Not flat: some neighbour is higher or lower (banded biomes show their layers).
    pub sloped: bool,
    /// World position `(x, z)`.
    pub at: (i32, i32),
}

/// Plain heights of a column and of the ring of blocks around it, indexed `[z + 1][x + 1]`.
pub struct Heights(pub [[i32; 18]; 18]);

impl Heights {
    pub fn at(&self, lx: i32, lz: i32) -> i32 {
        self.0[(lz + 1) as usize][(lx + 1) as usize]
    }

    /// Slopes of `slope` blocks or more to a neighbour stay bare rock, like cliffs.
    pub fn is_steep(&self, lx: i32, lz: i32, slope: i32) -> bool {
        let here = self.at(lx, lz);
        [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .iter()
            .any(|(dx, dz)| (self.at(lx + dx, lz + dz) - here).abs() >= slope)
    }
}

pub struct SurfaceRules<'a> {
    pub content: &'a WorldgenContent,
    pub seed: Seed,
    /// Waviness of the coloured layers of banded biomes.
    pub bands: &'a Noise,
}

impl SurfaceRules<'_> {
    /// Top block of the ground at height `y`: beach sand near the sea, rocky or snowy
    /// above peak heights, bare rock (or coloured layers) on steep slopes.
    pub fn top_block(&self, ground: &Ground<'_>, y: i32) -> BlockId {
        if y <= self.content.terrain.sea_level + 1 {
            return ground.biome.underwater;
        }
        match peak(ground.biome, y) {
            Some(block) => block,
            None => match (&ground.biome.bands, ground.sloped) {
                (Some(bands), true) => self.band(bands, ground.at, y),
                _ if ground.steep => ground.biome.steep.unwrap_or(self.content.terrain.stone),
                _ => ground.biome.surface,
            },
        }
    }

    fn below_surface_block(&self, ground: &Ground<'_>, surface_y: i32, y: i32) -> BlockId {
        let biome = ground.biome;
        if peak(biome, surface_y).is_some() {
            self.content.terrain.stone
        } else if let (Some(bands), true) = (&biome.bands, ground.sloped) {
            self.band(bands, ground.at, y)
        } else if ground.steep {
            self.content.terrain.stone
        } else if surface_y <= self.content.terrain.sea_level + 1 {
            biome.underwater
        } else {
            biome.subsurface
        }
    }

    /// Deeper than the soil: the biome's coloured layers, if any, else `None` (rock).
    fn layer_block(
        &self,
        ground: &Ground<'_>,
        surface_y: i32,
        depth: i32,
        y: i32,
    ) -> Option<BlockId> {
        let bands = ground.biome.bands.as_ref()?;
        let dry = surface_y > self.content.terrain.sea_level + 1;
        (dry && depth < bands.depth).then(|| self.band(bands, ground.at, y))
    }

    fn band(&self, bands: &Bands, (x, z): (i32, i32), y: i32) -> BlockId {
        let shift = (self.bands.at(f64::from(x), f64::from(z)) * bands.waviness).round() as i32;
        let index = (y + shift).rem_euclid(bands.blocks.len() as i32) as usize;
        bands.blocks[index]
    }

    /// Fills one block column top-down and returns the height of its highest solid block.
    /// Water only fills what is open to the sky: pockets under arches stay dry.
    pub fn fill(
        &self,
        column: &mut Column,
        (lx, lz): (usize, usize),
        ground: &Ground<'_>,
        grid: Option<&Grid>,
    ) -> i32 {
        let (sample, biome) = (ground.sample, ground.biome);
        let terrain = &self.content.terrain;
        let (x, z) = (column.pos.x * 16 + lx as i32, column.pos.z * 16 + lz as i32);
        let reach = sample.overhang.ceil() as i32;
        let solid = |y: i32| match grid {
            Some(grid) if reach > 0 && y >= sample.height - reach => {
                is_solid(sample.height, sample.overhang, y, grid.at(lx, y, lz))
            }
            _ => y <= sample.height,
        };
        let jitter = |salt: i32| hash(self.seed, x, salt, z);
        let deep_level = terrain.deep_level + (jitter(1) % 5) as i32 - 2;
        let soil_depth = biome.subsurface_depth as i32;
        let (cx, cz) = (lx as u32, lz as u32);
        let top = (sample.height + reach).max(terrain.sea_level);
        let mut highest = None;
        let mut since_open = 0;
        let mut surface_y = top;
        for y in (0..=top).rev() {
            if y > 0 && !solid(y) {
                since_open = 0;
                if y <= terrain.sea_level && highest.is_none() {
                    let frozen = biome.frozen && y == terrain.sea_level;
                    column.set(cx, y, cz, if frozen { terrain.ice } else { terrain.water });
                }
                continue;
            }
            highest.get_or_insert(y);
            if since_open == 0 {
                surface_y = y;
            }
            let block = if y == 0 || (y <= 3 && jitter(y + 10) % (y as u32 + 1) == 0) {
                terrain.bedrock
            } else if since_open == 0 {
                self.top_block(ground, y)
            } else if since_open < soil_depth {
                self.below_surface_block(ground, surface_y, y)
            } else if let Some(block) = self.layer_block(ground, surface_y, since_open, y) {
                block
            } else if y < deep_level {
                terrain.deep_stone
            } else {
                terrain.stone
            };
            column.set(cx, y, cz, block);
            since_open += 1;
        }
        highest.unwrap_or(0)
    }
}

fn peak(biome: &Biome, height: i32) -> Option<BlockId> {
    biome
        .peaks
        .iter()
        .rev()
        .find(|(from, _)| height >= *from)
        .map(|(_, block)| *block)
}

fn hash(seed: Seed, x: i32, y: i32, z: i32) -> u32 {
    let mut h = seed.0 ^ (x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= (y as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= (z as u32 as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    (h ^ (h >> 29)) as u32
}
