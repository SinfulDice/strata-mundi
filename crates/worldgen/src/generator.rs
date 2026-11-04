use crate::Seed;
use crate::caves::Caves;
use crate::climate::{ClimateMaps, Sample};
use crate::content::{Biome, WorldgenContent};
use crate::features::{place_ores, place_surface_features};
use crate::surface::{Ground, Heights, SurfaceRules};
use crate::terrain::{Grid, Overhangs};
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
    overhangs: Overhangs,
    caves: Caves,
}

/// Plain samples and biomes of the 16×16 blocks of a column, indexed `[z][x]`.
type ColumnSamples<'a> = [[(Sample, &'a Biome); 16]; 16];

impl Generator {
    pub fn new(seed: Seed, content: Arc<WorldgenContent>) -> Self {
        Self {
            maps: ClimateMaps::new(seed, &content),
            overhangs: Overhangs::new(seed, content.terrain.relief.overhang_scale),
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

    /// Height of the highest solid block at `(x, z)`, overhangs included.
    pub fn surface_height(&self, x: i32, z: i32) -> i32 {
        self.surface_height_at(&self.maps.sample(x, z), x, z)
    }

    /// Same as `surface_height`, for a point already sampled.
    pub fn surface_height_at(&self, sample: &Sample, x: i32, z: i32) -> i32 {
        self.overhangs.surface(x, z, sample.height, sample.overhang)
    }

    /// The block generated on top of the ground at `(x, height, z)`, before caves.
    pub fn ground_block(&self, x: i32, z: i32, sample: &Sample, biome: &Biome, y: i32) -> BlockId {
        let slope = self.content.terrain.relief.steep_slope;
        let steep = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dz)| {
            (self.maps.sample(x + dx, z + dz).height - sample.height).abs() >= slope
        });
        let ground = Ground {
            sample,
            biome,
            steep,
        };
        self.rules().top_block(&ground, y)
    }

    fn rules(&self) -> SurfaceRules<'_> {
        SurfaceRules {
            content: &self.content,
            seed: self.seed,
        }
    }

    #[tracing::instrument(skip(self), level = "trace")]
    pub fn generate(&self, pos: ColumnPos) -> Column {
        let mut column = Column::empty(pos);
        let origin = (pos.x * SECTION_SIZE, pos.z * SECTION_SIZE);
        let samples: ColumnSamples<'_> = std::array::from_fn(|lz| {
            std::array::from_fn(|lx| self.sample(origin.0 + lx as i32, origin.1 + lz as i32))
        });
        let ring = self.ring(origin, &samples);
        let grid = self.overhang_grid(origin, &samples);
        let rules = self.rules();
        let slope = self.content.terrain.relief.steep_slope;
        let mut tops = [[0; 16]; 16];
        for (lz, row) in samples.iter().enumerate() {
            for (lx, (sample, biome)) in row.iter().enumerate() {
                let ground = Ground {
                    sample,
                    biome,
                    steep: ring.plain.is_steep(lx as i32, lz as i32, slope),
                };
                tops[lz][lx] = rules.fill(&mut column, (lx, lz), &ground, grid.as_ref());
            }
        }
        let terrain = &self.content.terrain;
        if terrain.caves {
            let roofs = self.cave_roofs(&tops, &ring.lowest);
            self.caves.carve(&mut column, &roofs, terrain.bedrock);
        }
        place_ores(self, &mut column);
        place_surface_features(self, &mut column);
        column
    }

    /// Heights of the column and of the blocks around it: plain (for slopes) and lowest
    /// possible (for cave roofs, overhangs carved off).
    fn ring(&self, origin: (i32, i32), samples: &ColumnSamples<'_>) -> Ring {
        let mut plain = [[0; 18]; 18];
        let mut lowest = [[0; 18]; 18];
        for (rz, (plain_row, lowest_row)) in plain.iter_mut().zip(&mut lowest).enumerate() {
            for (rx, (plain, lowest)) in plain_row.iter_mut().zip(lowest_row.iter_mut()).enumerate()
            {
                let (lx, lz) = (rx as i32 - 1, rz as i32 - 1);
                let inside = (0..16).contains(&lx) && (0..16).contains(&lz);
                let sample = if inside {
                    samples[lz as usize][lx as usize].0
                } else {
                    self.maps.sample(origin.0 + lx, origin.1 + lz)
                };
                *plain = sample.height;
                *lowest = sample.height - sample.overhang.ceil() as i32;
            }
        }
        Ring {
            plain: Heights(plain),
            lowest: Heights(lowest),
        }
    }

    fn overhang_grid(&self, origin: (i32, i32), samples: &ColumnSamples<'_>) -> Option<Grid> {
        let flat = samples.iter().flatten().map(|(s, _)| s);
        let (mut low, mut high) = (i32::MAX, i32::MIN);
        for sample in flat.filter(|s| s.overhang > 0.0) {
            let reach = sample.overhang.ceil() as i32;
            low = low.min(sample.height - reach);
            high = high.max(sample.height + reach);
        }
        (low <= high).then(|| self.overhangs.grid(origin, low, high))
    }

    /// Highest y tunnels may reach under each block: a few blocks below the lowest ground
    /// around it (more below seas and rivers), so caves never open into water from the side.
    fn cave_roofs(&self, tops: &[[i32; 16]; 16], ring: &Heights) -> [[i32; 16]; 16] {
        let sea_level = self.content.terrain.sea_level;
        let height_at = |lx: i32, lz: i32| {
            if (0..16).contains(&lx) && (0..16).contains(&lz) {
                tops[lz as usize][lx as usize]
            } else {
                ring.at(lx, lz)
            }
        };
        std::array::from_fn(|lz| {
            std::array::from_fn(|lx| {
                let lowest = (-1..=1)
                    .flat_map(|dz| (-1..=1).map(move |dx| (dx, dz)))
                    .map(|(dx, dz)| height_at(lx as i32 + dx, lz as i32 + dz))
                    .min()
                    .unwrap_or(0);
                lowest - if lowest < sea_level + 2 { 9 } else { 4 }
            })
        })
    }

    /// A welcoming place to start: a spawn biome (plains, forest) on dry land, with water
    /// close by. Searched in a spiral from the origin; falls back to any dry spawn biome.
    pub fn spawn_point(&self) -> BlockPos {
        crate::spawn::find(self)
    }
}

struct Ring {
    plain: Heights,
    lowest: Heights,
}
