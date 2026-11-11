use crate::caves::CLEARANCE;
use crate::content::{Biome, Feature};
use crate::generator::Generator;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE};

const STEP: i32 = 32;
const MAX_RADIUS: i32 = 4096;
/// Water must be within this distance of a good spawn.
const WATER_RANGE: i32 = 96;
/// Caves may not open this many blocks under a spawn or its neighbours.
const CAVE_MARGIN: i32 = 4;

/// Falls back to a spot without water nearby, then to any dry land (types forcing an
/// unwelcoming biome); flat worlds take the first free spot.
pub fn find(generator: &Generator) -> BlockPos {
    let terrain = &generator.content().terrain;
    let sea_level = terrain.sea_level;
    let flat = terrain.flat.is_some();
    let (mut fallback, mut dry) = (None, None);
    for (x, z) in spiral() {
        let (sample, biome) = generator.sample(x, z);
        if !flat && sample.height <= sea_level + 1 {
            continue;
        }
        let welcoming = biome.spawn && has_trees(generator, biome);
        if !welcoming && dry.is_some() {
            continue;
        }
        let height = generator.surface_height_at(&sample, x, z);
        if near_cave_entrance(generator, x, z, height) {
            continue;
        }
        let Some(spot) = free_standing(generator, x, z, height) else {
            continue;
        };
        if flat {
            return spot;
        }
        if !welcoming {
            dry = Some(spot);
        } else if has_water_nearby(generator, x, z, sea_level) {
            return spot;
        } else {
            fallback.get_or_insert(spot);
        }
    }
    fallback.or(dry).unwrap_or(BlockPos::new(
        0,
        generator.surface_height(0, 0).max(sea_level) + 1,
        0,
    ))
}

fn has_trees(generator: &Generator, biome: &Biome) -> bool {
    let content = generator.content();
    let flat_without = content
        .terrain
        .flat
        .as_ref()
        .is_some_and(|flat| !flat.features);
    !flat_without
        && biome
            .features
            .iter()
            .any(|(index, _)| matches!(content.features[*index], Feature::Tree(_)))
}

/// Whether a cave may open under the spot or around it, even a few blocks down.
fn near_cave_entrance(generator: &Generator, x: i32, z: i32, height: i32) -> bool {
    let ravines = generator.ravines_around((x - 1, z - 1), (x + 1, z + 1));
    (-1..=1)
        .flat_map(|dz| (-1..=1).map(move |dx| (x + dx, z + dz)))
        .any(|(nx, nz)| {
            let top = if (nx, nz) == (x, z) {
                height
            } else {
                generator.surface_height(nx, nz)
            };
            ravines.surface_reaches(nx, nz, CLEARANCE)
                || generator.tunnel_may_open(nx, nz, top - CAVE_MARGIN..=top)
        })
}

/// First spot above the ground with two blocks of air over a solid block, so the player
/// never starts inside a tree trunk, under leaves or over a hole.
fn free_standing(generator: &Generator, x: i32, z: i32, height: i32) -> Option<BlockPos> {
    let column = generator.generate(ColumnPos {
        x: x.div_euclid(SECTION_SIZE),
        z: z.div_euclid(SECTION_SIZE),
    });
    let (lx, lz) = (
        x.rem_euclid(SECTION_SIZE) as u32,
        z.rem_euclid(SECTION_SIZE) as u32,
    );
    let terrain = &generator.content().terrain;
    let lava = terrain.caves.as_ref().map(|caves| caves.lava);
    let air = |y: i32| column.get(lx, y, lz) == BlockId::AIR;
    let solid = |y: i32| {
        let block = column.get(lx, y, lz);
        block != BlockId::AIR && block != terrain.water && Some(block) != lava
    };
    (height + 1..height + 40)
        .find(|&y| air(y) && air(y + 1))
        .filter(|&y| solid(y - 1))
        .map(|y| BlockPos::new(x, y, z))
}

fn has_water_nearby(generator: &Generator, x: i32, z: i32, sea_level: i32) -> bool {
    (1..=WATER_RANGE / 16).any(|ring| {
        let d = ring * 16;
        [
            (d, 0),
            (-d, 0),
            (0, d),
            (0, -d),
            (d, d),
            (-d, -d),
            (d, -d),
            (-d, d),
        ]
        .iter()
        .any(|(dx, dz)| generator.sample(x + dx, z + dz).0.height < sea_level)
    })
}

/// Grid points on growing square rings around the origin (corners appear twice, which
/// only costs a duplicate check).
fn spiral() -> impl Iterator<Item = (i32, i32)> {
    let rings = MAX_RADIUS / STEP;
    std::iter::once((0, 0)).chain((1..=rings).flat_map(|ring| {
        (-ring..=ring).flat_map(move |i| {
            [(i, -ring), (i, ring), (-ring, i), (ring, i)].map(|(a, b)| (a * STEP, b * STEP))
        })
    }))
}
