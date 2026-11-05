//! Patches of small plants (grass, flowers, mushrooms). Like trees, a column also grows
//! the plants of patches centered in its eight neighbours, so patches cross borders.

use crate::content::{Feature, MAX_PATCH_RADIUS, Plants};
use crate::generator::Generator;
use crate::rng::Rng;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{ColumnPos, SECTION_SIZE};

/// Patch centers tried per column; a biome's `per_column` is spread over them.
pub const PATCH_ATTEMPTS: u32 = 8;
const PATCH_SALT: u64 = 3000;

/// `tops` holds the height of the ground under each block of the column.
pub fn place_plants(generator: &Generator, column: &mut Column, tops: &[[i32; 16]; 16]) {
    for dz in -1..=1 {
        for dx in -1..=1 {
            let source = ColumnPos {
                x: column.pos.x + dx,
                z: column.pos.z + dz,
            };
            place_patches_from(generator, source, column, tops);
        }
    }
}

fn place_patches_from(
    generator: &Generator,
    source: ColumnPos,
    column: &mut Column,
    tops: &[[i32; 16]; 16],
) {
    let content = generator.content();
    for attempt in 0..PATCH_ATTEMPTS {
        let mut rng = Rng::for_column(generator.seed(), source, PATCH_SALT + u64::from(attempt));
        let x = source.x * SECTION_SIZE + rng.range(0, 15);
        let z = source.z * SECTION_SIZE + rng.range(0, 15);
        if !reaches(column.pos, x, z) {
            continue;
        }
        let (sample, biome) = generator.sample(x, z);
        if sample.height < content.terrain.sea_level {
            continue;
        }
        for &(index, per_column) in &biome.features {
            let Some(Feature::Plants(plants)) = content.features.get(index) else {
                continue;
            };
            let salt = plants.salt.wrapping_add(u64::from(attempt));
            let mut rng = Rng::for_column(generator.seed(), source, salt);
            if rng.unit() * f64::from(PATCH_ATTEMPTS) < per_column {
                place_patch(plants, (x, z), &mut rng, column, tops);
            }
        }
    }
}

fn reaches(pos: ColumnPos, x: i32, z: i32) -> bool {
    let reach = MAX_PATCH_RADIUS as i32;
    let near = |c: i32, origin: i32| {
        let start = origin * SECTION_SIZE;
        (start - reach..start + SECTION_SIZE + reach).contains(&c)
    };
    near(x, pos.x) && near(z, pos.z)
}

/// Draws every plant of the patch (whatever column it falls in, to stay deterministic) and
/// sets those inside `column` that stand on a block of `on` with air above.
fn place_patch(
    plants: &Plants,
    (x, z): (i32, i32),
    rng: &mut Rng,
    column: &mut Column,
    tops: &[[i32; 16]; 16],
) {
    let total: u32 = plants.blocks.iter().map(|(_, weight)| weight).sum();
    if total == 0 {
        return;
    }
    let r = plants.radius as i32;
    let count = rng.range(plants.count.0 as i32, plants.count.1 as i32);
    for _ in 0..count {
        let (dx, dz) = (rng.range(-r, r), rng.range(-r, r));
        let block = weighted(&plants.blocks, (rng.next_u64() % u64::from(total)) as u32);
        let lx = x + dx - column.pos.x * SECTION_SIZE;
        let lz = z + dz - column.pos.z * SECTION_SIZE;
        if !(0..16).contains(&lx) || !(0..16).contains(&lz) {
            continue;
        }
        let (cx, cz) = (lx as u32, lz as u32);
        let ground = tops[lz as usize][lx as usize];
        let open = column.get(cx, ground + 1, cz) == BlockId::AIR;
        if open && plants.on.contains(&column.get(cx, ground, cz)) {
            column.set(cx, ground + 1, cz, block);
        }
    }
}

fn weighted(blocks: &[(BlockId, u32)], mut pick: u32) -> BlockId {
    for &(block, weight) in blocks {
        if pick < weight {
            return block;
        }
        pick -= weight;
    }
    BlockId::AIR
}
