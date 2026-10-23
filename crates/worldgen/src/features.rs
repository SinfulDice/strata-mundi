//! Ore veins, trees and pillars. A column also places the parts of features rooted in
//! its eight neighbours, so trees cross borders seamlessly whatever the loading order.

use crate::content::Feature;
use crate::defs::Crown;
use crate::generator::Generator;
use crate::rng::Rng;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{ColumnPos, SECTION_SIZE};

/// Placement attempts per column; a biome's `per_column` is spread over them.
const ATTEMPTS: u32 = 16;
const ORE_SALT: u64 = 500;
const SURFACE_SALT: u64 = 1000;

pub fn place_ores(generator: &Generator, column: &mut Column) {
    let content = generator.content();
    let terrain = &content.terrain;
    for (index, &feature) in terrain.ores.iter().enumerate() {
        let Feature::Ore {
            block,
            deep_block,
            vein_size,
            per_column,
            min_y,
            max_y,
        } = content.features[feature]
        else {
            continue;
        };
        let mut rng = Rng::for_column(generator.seed(), column.pos, ORE_SALT + index as u64);
        for _ in 0..rng.count(per_column) {
            let mut pos = [rng.range(0, 15), rng.range(min_y, max_y), rng.range(0, 15)];
            for _ in 0..vein_size {
                let host = column.get(pos[0] as u32, pos[1], pos[2] as u32);
                let ore = match host {
                    h if h == terrain.stone => Some(block),
                    h if h == terrain.deep_stone => Some(deep_block.unwrap_or(block)),
                    _ => None,
                };
                if let Some(ore) = ore {
                    column.set(pos[0] as u32, pos[1], pos[2] as u32, ore);
                }
                let axis = rng.range(0, 2) as usize;
                pos[axis] += if rng.unit() < 0.5 { -1 } else { 1 };
                // A vein reaching the border stops instead of piling up along it.
                if !(0..16).contains(&pos[0]) || !(0..16).contains(&pos[2]) {
                    break;
                }
            }
        }
    }
}

pub fn place_surface_features(generator: &Generator, column: &mut Column) {
    for dz in -1..=1 {
        for dx in -1..=1 {
            let source = ColumnPos {
                x: column.pos.x + dx,
                z: column.pos.z + dz,
            };
            place_rooted_in(generator, source, column);
        }
    }
}

fn place_rooted_in(generator: &Generator, source: ColumnPos, column: &mut Column) {
    let content = generator.content();
    let mut rng = Rng::for_column(generator.seed(), source, SURFACE_SALT);
    for _ in 0..ATTEMPTS {
        let x = source.x * SECTION_SIZE + rng.range(0, 15);
        let z = source.z * SECTION_SIZE + rng.range(0, 15);
        let mut roll = rng.unit() * f64::from(ATTEMPTS);
        let variant = rng.next_u64();
        let (sample, biome) = generator.sample(x, z);
        if sample.height < content.terrain.sea_level {
            continue;
        }
        let ground = generator.surface_block(&sample, biome);
        for &(feature, per_column) in &biome.features {
            roll -= per_column;
            if roll < 0.0 {
                let mut writer = Writer {
                    column,
                    x,
                    z,
                    base: sample.height + 1,
                };
                place(&content.features[feature], ground, variant, &mut writer);
                break;
            }
        }
    }
}

/// Writes the blocks of one feature that fall inside the column being generated.
struct Writer<'a> {
    column: &'a mut Column,
    x: i32,
    z: i32,
    base: i32,
}

impl Writer<'_> {
    /// `(dx, dy, dz)` from the feature origin. Only replaces air (and leaves, for logs).
    fn put(&mut self, offset: (i32, i32, i32), block: BlockId, over: Option<BlockId>) {
        let (x, z) = (self.x + offset.0, self.z + offset.2);
        let local = |c: i32, origin: i32| c - origin * SECTION_SIZE;
        let (lx, lz) = (local(x, self.column.pos.x), local(z, self.column.pos.z));
        if !(0..16).contains(&lx) || !(0..16).contains(&lz) {
            return;
        }
        let y = self.base + offset.1;
        let current = self.column.get(lx as u32, y, lz as u32);
        if current == BlockId::AIR || Some(current) == over {
            self.column.set(lx as u32, y, lz as u32, block);
        }
    }
}

fn place(feature: &Feature, ground: BlockId, variant: u64, writer: &mut Writer<'_>) {
    let pick = |(low, high): (u32, u32)| low as i32 + (variant % u64::from(high - low + 1)) as i32;
    match feature {
        Feature::Tree {
            log,
            leaves,
            height,
            crown,
            on,
        } if on.contains(&ground) => {
            let trunk = pick(*height);
            crown_shape(*crown, trunk, variant, |offset| {
                writer.put(offset, *leaves, None)
            });
            for y in 0..trunk {
                writer.put((0, y, 0), *log, Some(*leaves));
            }
        }
        Feature::Pillar { block, height, on } if on.contains(&ground) => {
            for y in 0..pick(*height) {
                writer.put((0, y, 0), *block, None);
            }
        }
        _ => {}
    }
}

fn crown_shape(crown: Crown, trunk: i32, variant: u64, mut leaf: impl FnMut((i32, i32, i32))) {
    match crown {
        Crown::Round { radius } => {
            let r = radius as i32;
            for dy in -2..=1 {
                let layer_r = if dy >= 0 { r - 1 } else { r };
                for dx in -layer_r..=layer_r {
                    for dz in -layer_r..=layer_r {
                        let corner = dx.abs() == layer_r && dz.abs() == layer_r;
                        let trimmed = corner && (variant >> ((dx + dz + dy + 8) % 32)) & 1 == 0;
                        if !trimmed || dy < -1 {
                            leaf((dx, trunk + dy, dz));
                        }
                    }
                }
            }
            leaf((0, trunk + 1, 0));
        }
        Crown::Cone { radius } => {
            let r = radius as i32;
            for dy in 2..=trunk {
                let from_top = trunk - dy;
                let layer_r =
                    ((from_top + 1) / 2).min(r) - i32::from(from_top % 2 == 1 && from_top > 2);
                for dx in -layer_r..=layer_r {
                    for dz in -layer_r..=layer_r {
                        if dx.abs() + dz.abs() <= layer_r + 1 {
                            leaf((dx, dy, dz));
                        }
                    }
                }
            }
            leaf((0, trunk, 0));
            leaf((0, trunk + 1, 0));
        }
    }
}
