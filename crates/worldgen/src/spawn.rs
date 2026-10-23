use crate::content::Feature;
use crate::generator::Generator;
use strata_core::content::BlockId;
use strata_core::coords::{BlockPos, ColumnPos, SECTION_SIZE};

const STEP: i32 = 32;
const MAX_RADIUS: i32 = 4096;
/// Water must be within this distance of a good spawn.
const WATER_RANGE: i32 = 96;

pub fn find(generator: &Generator) -> BlockPos {
    let sea_level = generator.content().terrain.sea_level;
    let mut fallback = None;
    for (x, z) in spiral() {
        let (sample, biome) = generator.sample(x, z);
        let has_trees = biome
            .features
            .iter()
            .any(|(index, _)| matches!(generator.content().features[*index], Feature::Tree { .. }));
        if !biome.spawn || !has_trees || sample.height <= sea_level + 1 {
            continue;
        }
        if has_water_nearby(generator, x, z, sea_level) {
            return free_standing(generator, x, z, sample.height);
        }
        fallback.get_or_insert((x, z, sample.height));
    }
    fallback.map_or(BlockPos::new(0, sea_level + 2, 0), |(x, z, height)| {
        free_standing(generator, x, z, height)
    })
}

/// First spot above the ground with two blocks of air, so the player never starts
/// inside a tree trunk or under leaves.
fn free_standing(generator: &Generator, x: i32, z: i32, height: i32) -> BlockPos {
    let column = generator.generate(ColumnPos {
        x: x.div_euclid(SECTION_SIZE),
        z: z.div_euclid(SECTION_SIZE),
    });
    let (lx, lz) = (
        x.rem_euclid(SECTION_SIZE) as u32,
        z.rem_euclid(SECTION_SIZE) as u32,
    );
    let free =
        |y: i32| column.get(lx, y, lz) == BlockId::AIR && column.get(lx, y + 1, lz) == BlockId::AIR;
    let y = (height + 1..height + 40)
        .find(|&y| free(y))
        .unwrap_or(height + 1);
    BlockPos::new(x, y, z)
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
