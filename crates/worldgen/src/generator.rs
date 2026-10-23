use crate::Seed;
use crate::caves::Caves;
use crate::climate::{ClimateMaps, Sample};
use crate::content::{Biome, WorldgenContent};
use crate::features::{place_ores, place_surface_features};
use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE};

/// Builds columns from the seed. Pure: any column can be generated at any time,
/// on any thread, in any order, always with the same result.
pub struct Generator {
    seed: Seed,
    content: Arc<WorldgenContent>,
    maps: ClimateMaps,
    caves: Caves,
}

impl Generator {
    pub fn new(seed: Seed, content: Arc<WorldgenContent>) -> Self {
        Self {
            maps: ClimateMaps::new(seed, &content),
            caves: Caves::new(seed),
            seed,
            content,
        }
    }

    pub fn seed(&self) -> Seed {
        self.seed
    }

    pub fn content(&self) -> &WorldgenContent {
        &self.content
    }

    pub fn sample(&self, x: i32, z: i32) -> (Sample, &Biome) {
        let sample = self.maps.sample(x, z);
        let biome = self.maps.biome(&self.content, x, z, &sample);
        (sample, biome)
    }

    #[tracing::instrument(skip(self), level = "trace")]
    pub fn generate(&self, pos: ColumnPos) -> Column {
        let mut column = Column::empty(pos);
        let mut heights = [[0; 16]; 16];
        for (lz, row) in heights.iter_mut().enumerate() {
            for (lx, height) in row.iter_mut().enumerate() {
                let x = pos.x * SECTION_SIZE + lx as i32;
                let z = pos.z * SECTION_SIZE + lz as i32;
                let (sample, biome) = self.sample(x, z);
                self.fill_terrain(&mut column, (lx as u32, lz as u32), (x, z), &sample, biome);
                *height = sample.height;
            }
        }
        let terrain = &self.content.terrain;
        if terrain.caves {
            let roofs = self.cave_roofs(pos, &heights);
            self.caves.carve(&mut column, &roofs, terrain.bedrock);
        }
        place_ores(self, &mut column);
        place_surface_features(self, &mut column);
        column
    }

    /// Highest y tunnels may reach under each block: a few blocks below the lowest ground
    /// around it (more below seas), so caves never open into water from the side.
    fn cave_roofs(&self, pos: ColumnPos, heights: &[[i32; 16]; 16]) -> [[i32; 16]; 16] {
        let sea_level = self.content.terrain.sea_level;
        let height_at = |lx: i32, lz: i32| {
            if (0..16).contains(&lx) && (0..16).contains(&lz) {
                heights[lz as usize][lx as usize]
            } else {
                self.maps
                    .sample(pos.x * SECTION_SIZE + lx, pos.z * SECTION_SIZE + lz)
                    .height
            }
        };
        std::array::from_fn(|lz| {
            std::array::from_fn(|lx| {
                let lowest = (-1..=1)
                    .flat_map(|dz| (-1..=1).map(move |dx| (dx, dz)))
                    .map(|(dx, dz)| height_at(lx as i32 + dx, lz as i32 + dz))
                    .min()
                    .unwrap_or(0);
                lowest - if lowest < sea_level { 9 } else { 4 }
            })
        })
    }

    /// Top block of the ground: beach sand near the sea, rocky or snowy above peak heights.
    pub fn surface_block(&self, sample: &Sample, biome: &Biome) -> BlockId {
        if sample.height <= self.content.terrain.sea_level + 1 {
            return biome.underwater;
        }
        peak(biome, sample.height).unwrap_or(biome.surface)
    }

    fn fill_terrain(
        &self,
        column: &mut Column,
        (lx, lz): (u32, u32),
        (x, z): (i32, i32),
        sample: &Sample,
        biome: &Biome,
    ) {
        let terrain = &self.content.terrain;
        let height = sample.height;
        let surface = self.surface_block(sample, biome);
        let below_surface = if peak(biome, height).is_some() {
            terrain.stone
        } else if height <= terrain.sea_level + 1 {
            biome.underwater
        } else {
            biome.subsurface
        };
        let jitter = |salt: i32| hash(self.seed, x, salt, z);
        let deep_level = terrain.deep_level + (jitter(1) % 5) as i32 - 2;
        let soil_depth = biome.subsurface_depth as i32;
        for y in 0..=height {
            let block = if y == 0 || (y <= 3 && jitter(y + 10) % (y as u32 + 1) == 0) {
                terrain.bedrock
            } else if y == height {
                surface
            } else if y > height - soil_depth {
                below_surface
            } else if y < deep_level {
                terrain.deep_stone
            } else {
                terrain.stone
            };
            column.set(lx, y, lz, block);
        }
        for y in height + 1..=terrain.sea_level {
            let frozen = biome.frozen && y == terrain.sea_level;
            column.set(lx, y, lz, if frozen { terrain.ice } else { terrain.water });
        }
    }

    /// A welcoming place to start: a spawn biome (plains, forest) on dry land, with water
    /// close by. Searched in a spiral from the origin; falls back to any dry spawn biome.
    pub fn spawn_point(&self) -> BlockPos {
        crate::spawn::find(self)
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
