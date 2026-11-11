use crate::Seed;
use crate::SettingsError;
use crate::caves::{CLEARANCE, Caves, Nearby, Roofs};
use crate::climate::{ClimateMaps, Sample};
use crate::content::{Biome, Flat, MAX_CROWN, WorldgenContent};
use crate::deposits::{Deposits, Vein};
use crate::features::{place_ores, place_surface_features};
use crate::noise::Noise;
use crate::plants::place_plants;
use crate::surface::{Ground, Heights, SurfaceRules};
use crate::terrain::{Grid, Overhangs};
use std::ops::RangeInclusive;
use std::sync::Arc;
use strata_core::chunk::{BiomeMap, Column};
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE};
use strata_core::generation::WorldgenSettings;

/// Builds columns from the seed. Pure: any column can be generated at any time,
/// on any thread, in any order, always with the same result.
pub struct Generator {
    seed: Seed,
    content: Arc<WorldgenContent>,
    maps: ClimateMaps,
    overhangs: Overhangs,
    caves: Option<Caves>,
    deposits: Deposits,
    bands: Noise,
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
            bands: Noise::new(seed, 40, 1.0 / 48.0, 2),
            seed,
            content,
        }
    }

    /// The generator of a world created with `settings` (the default ones keep the pack's
    /// world exactly).
    pub fn with_settings(
        seed: Seed,
        content: &Arc<WorldgenContent>,
        settings: &WorldgenSettings,
    ) -> Result<Self, SettingsError> {
        if *settings == WorldgenSettings::default() {
            return Ok(Self::new(seed, Arc::clone(content)));
        }
        let changed = content.with_settings(settings)?;
        Ok(Self::new(seed, Arc::new(changed)))
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
        if let Some(flat) = &self.content.terrain.flat {
            return flat.layers.last().map_or(BlockId::AIR, |(block, _)| *block);
        }
        let slope = self.content.terrain.relief.steep_slope;
        let rise = [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .iter()
            .map(|(dx, dz)| (self.maps.sample(x + dx, z + dz).height - sample.height).abs())
            .max()
            .unwrap_or(0);
        let ground = Ground {
            sample,
            biome,
            steep: rise >= slope,
            sloped: rise >= 1,
            at: (x, z),
        };
        self.rules().top_block(&ground, y)
    }

    fn rules(&self) -> SurfaceRules<'_> {
        SurfaceRules {
            content: &self.content,
            seed: self.seed,
            bands: &self.bands,
        }
    }

    pub fn generate(&self, pos: ColumnPos) -> Column {
        self.generate_with_biomes(pos).0
    }

    /// The column and the biome of each of its block columns.
    #[tracing::instrument(skip(self), level = "trace")]
    pub fn generate_with_biomes(&self, pos: ColumnPos) -> (Column, BiomeMap) {
        let mut column = Column::empty(pos);
        let origin = (pos.x * SECTION_SIZE, pos.z * SECTION_SIZE);
        let samples: ColumnSamples<'_> = std::array::from_fn(|lz| {
            std::array::from_fn(|lx| self.sample(origin.0 + lx as i32, origin.1 + lz as i32))
        });
        let biomes = BiomeMap::from_fn(|lx, lz| samples[lz as usize][lx as usize].1.index);
        let ring = self.ring(origin, &samples);
        let flat = self.content.terrain.flat.as_ref();
        let tops = match flat {
            Some(flat) => fill_flat(&mut column, flat),
            None => self.fill_relief(&mut column, &samples, &ring),
        };
        let terrain = &self.content.terrain;
        // Wide enough for the trees rooted in the neighbouring columns.
        let reach = MAX_CROWN as i32 + 1;
        let ravines = self.ravines_around(
            (origin.0 - reach, origin.1 - reach),
            (origin.0 + 15 + reach, origin.1 + 15 + reach),
        );
        if let (Some(caves), Some(settings)) = (&self.caves, &terrain.caves) {
            let roofs = self.cave_roofs(&tops, &ring.lowest);
            let keep = [terrain.bedrock, terrain.water, terrain.ice, settings.lava];
            caves.carve(&mut column, &roofs, &keep, &ravines);
            self.bare_ravine_walls(&mut column, &samples, &tops, &ravines);
        }
        place_ores(self, &mut column);
        let features = flat.is_none_or(|flat| flat.features);
        if features {
            place_surface_features(self, &mut column, &ravines);
        }
        // After trees, so that no trunk stands on a deposit's sign.
        self.deposits
            .place(self.seed, &self.content, &mut column, &tops);
        if features {
            place_plants(self, &mut column, &tops);
        }
        (column, biomes)
    }

    /// Ground, water and soil of each block column; returns the height of the ground.
    fn fill_relief(
        &self,
        column: &mut Column,
        samples: &ColumnSamples<'_>,
        ring: &Ring,
    ) -> [[i32; 16]; 16] {
        let origin = (column.pos.x * SECTION_SIZE, column.pos.z * SECTION_SIZE);
        let grid = self.overhang_grid(origin, samples);
        let rules = self.rules();
        let slope = self.content.terrain.relief.steep_slope;
        let mut tops = [[0; 16]; 16];
        for (lz, row) in samples.iter().enumerate() {
            for (lx, (sample, biome)) in row.iter().enumerate() {
                let ground = Ground {
                    sample,
                    biome,
                    steep: ring.plain.is_steep(lx as i32, lz as i32, slope),
                    sloped: ring.plain.is_steep(lx as i32, lz as i32, 1),
                    at: (origin.0 + lx as i32, origin.1 + lz as i32),
                };
                tops[lz][lx] = rules.fill(column, (lx, lz), &ground, grid.as_ref());
            }
        }
        tops
    }

    /// Soil left on the walls and rims of ravines open at the surface turns to the
    /// biome's rock: no grass or dirt hanging over the void.
    fn bare_ravine_walls(
        &self,
        column: &mut Column,
        samples: &ColumnSamples<'_>,
        tops: &[[i32; 16]; 16],
        ravines: &Nearby,
    ) {
        let origin = (column.pos.x * SECTION_SIZE, column.pos.z * SECTION_SIZE);
        let stone = self.content.terrain.stone;
        // Own height, not the roof: a rim next to a ravine on dry land may itself be near
        // lower ground.
        let dry = self.content.terrain.sea_level + DRY_LAND;
        let rims = Caves::ravine_rims(origin, ravines);
        for (lz, row) in samples.iter().enumerate() {
            for (lx, (_, biome)) in row.iter().enumerate() {
                let top = tops[lz][lx];
                if top < dry || !rims[lz][lx] {
                    continue;
                }
                let soil = [biome.surface, biome.subsurface];
                let rock = biome.steep.filter(|b| !soil.contains(b)).unwrap_or(stone);
                let (cx, cz) = (lx as u32, lz as u32);
                for y in 0..=top {
                    if soil.contains(&column.get(cx, y, cz)) {
                        column.set(cx, y, cz, rock);
                    }
                }
            }
        }
    }

    /// The biomes of a column without generating it (columns read from a save).
    pub fn biome_map(&self, pos: ColumnPos) -> BiomeMap {
        let origin = (pos.x * SECTION_SIZE, pos.z * SECTION_SIZE);
        BiomeMap::from_fn(|lx, lz| {
            self.sample(origin.0 + lx as i32, origin.1 + lz as i32)
                .1
                .index
        })
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

    /// Whether a ravine, open at the surface or hidden, passes under `(x, z)`.
    pub fn in_ravine(&self, x: i32, z: i32) -> bool {
        self.ravines_around((x, z), (x, z))
            .inside(x, z)
            .next()
            .is_some()
    }

    /// Whether `(x, z)` is too close to a ravine open at the surface for anything to grow.
    pub fn near_open_ravine(&self, x: i32, z: i32) -> bool {
        self.ravines_around((x, z), (x, z))
            .surface_reaches(x, z, CLEARANCE)
    }

    /// The ravines that may come near the blocks from `min` to `max` (inclusive).
    pub(crate) fn ravines_around(&self, min: (i32, i32), max: (i32, i32)) -> Nearby {
        self.caves
            .as_ref()
            .map(|caves| caves.ravines_around(min, max))
            .unwrap_or_default()
    }

    /// Whether a tunnel may open a block of `ys` at `(x, z)`.
    pub fn tunnel_may_open(&self, x: i32, z: i32, ys: RangeInclusive<i32>) -> bool {
        self.caves
            .as_ref()
            .is_some_and(|caves| caves.tunnel_may_open(x, z, ys))
    }

    /// A welcoming place to start: a spawn biome (plains, forest) on dry land, with water
    /// close by. Searched in a spiral from the origin; falls back to any dry spawn biome.
    pub fn spawn_point(&self) -> BlockPos {
        crate::spawn::find(self)
    }
}

fn fill_flat(column: &mut Column, flat: &Flat) -> [[i32; 16]; 16] {
    let mut y = 0;
    for &(block, thickness) in &flat.layers {
        for _ in 0..thickness {
            for (lx, lz) in (0..16).flat_map(|lz| (0..16).map(move |lx| (lx, lz))) {
                column.set(lx, y, lz, block);
            }
            y += 1;
        }
    }
    [[flat.top(); 16]; 16]
}

struct Ring {
    plain: Heights,
    lowest: Heights,
}
