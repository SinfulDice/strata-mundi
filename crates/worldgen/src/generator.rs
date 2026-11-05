use crate::Seed;
use crate::caves::{Caves, Roofs};
use crate::climate::{ClimateMaps, Sample};
use crate::content::{Biome, WorldgenContent};
use crate::deposits::{Deposits, Vein};
use crate::features::{place_ores, place_surface_features};
use crate::plants::place_plants;
use crate::surface::{Ground, Heights, SurfaceRules};
use crate::terrain::{Grid, Overhangs};
use std::ops::RangeInclusive;
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
    caves: Option<Caves>,
    deposits: Deposits,
}

/// Ground at least this far above the sea, all around, counts as dry land.
const DRY_LAND: i32 = 3;
/// Rock kept between caves and the lowest ground around, near water.
const WATER_ROOF: i32 = 9;
const CAVERN_ROOF: i32 = 12;

/// Plain samples and biomes of the 16×16 blocks of a column, indexed `[z][x]`.
type ColumnSamples<'a> = [[(Sample, &'a Biome); 16]; 16];

impl Generator {
    pub fn new(seed: Seed, content: Arc<WorldgenContent>) -> Self {
        Self {
            maps: ClimateMaps::new(seed, &content),
            overhangs: Overhangs::new(seed, content.terrain.relief.overhang_scale),
            caves: content.terrain.caves.as_ref().map(|c| Caves::new(seed, c)),
            deposits: Deposits::new(seed),
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
        if let (Some(caves), Some(settings)) = (&self.caves, &terrain.caves) {
            let roofs = self.cave_roofs(&tops, &ring.lowest);
            let keep = [terrain.bedrock, terrain.water, terrain.ice, settings.lava];
            caves.carve(&mut column, &roofs, &keep);
        }
        place_ores(self, &mut column);
        self.deposits
            .place(self.seed, &self.content, &mut column, &tops);
        place_surface_features(self, &mut column);
        place_plants(self, &mut column, &tops);
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

    /// Highest y caves may reach under each block. On dry land, tunnels and ravines may
    /// open at the surface; near seas and rivers they stay well below the lowest ground
    /// around, so they never open into water. Caverns always keep a thick roof.
    fn cave_roofs(&self, tops: &[[i32; 16]; 16], ring: &Heights) -> Roofs {
        let sea_level = self.content.terrain.sea_level;
        let height_at = |lx: i32, lz: i32| {
            if (0..16).contains(&lx) && (0..16).contains(&lz) {
                tops[lz as usize][lx as usize]
            } else {
                ring.at(lx, lz)
            }
        };
        let lowest: [[i32; 16]; 16] = std::array::from_fn(|lz| {
            std::array::from_fn(|lx| {
                (-1..=1)
                    .flat_map(|dz| (-1..=1).map(move |dx| (dx, dz)))
                    .map(|(dx, dz)| height_at(lx as i32 + dx, lz as i32 + dz))
                    .min()
                    .unwrap_or(0)
            })
        });
        let tunnels = std::array::from_fn(|lz| {
            std::array::from_fn(|lx| {
                let low = lowest[lz][lx];
                if low >= sea_level + DRY_LAND {
                    tops[lz][lx]
                } else {
                    low - WATER_ROOF
                }
            })
        });
        let caverns = std::array::from_fn(|lz| {
            std::array::from_fn(|lx| (lowest[lz][lx] - CAVERN_ROOF).min(tunnels[lz][lx]))
        });
        Roofs { tunnels, caverns }
    }

    /// Giant ore veins with blocks possibly between `min` and `max` (horizontal block
    /// coordinates, inclusive).
    pub fn deposits_near(&self, min: (i32, i32), max: (i32, i32)) -> Vec<Vein> {
        Deposits::veins(self.seed, &self.content, min, max).collect()
    }

    /// Whether caves may open a block of `ys` at `(x, z)`.
    pub fn cave_may_open(&self, x: i32, z: i32, ys: RangeInclusive<i32>) -> bool {
        self.caves
            .as_ref()
            .is_some_and(|caves| caves.may_open(x, z, ys))
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
