//! Ore veins, trees and pillars. A column also places the parts of features rooted in
//! its eight neighbours, so trees cross borders seamlessly whatever the loading order.

use crate::caves::{CLEARANCE, MAX_MARGIN, Nearby};
use crate::content::{Feature, MAX_CROWN, WorldgenContent};
use crate::generator::Generator;
use crate::rng::Rng;
use crate::trees::{Part, Tree};
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

/// `ravines` holds the ravines near the column and the trees that may reach it.
pub fn place_surface_features(generator: &Generator, column: &mut Column, ravines: &Nearby) {
    let leaves = all_leaves(generator.content());
    for dz in -1..=1 {
        for dx in -1..=1 {
            let source = ColumnPos {
                x: column.pos.x + dx,
                z: column.pos.z + dz,
            };
            place_rooted_in(generator, (source, ravines), column, &leaves);
        }
    }
}

/// Leaves of every tree: a trunk grows through the crown of a neighbouring tree.
fn all_leaves(content: &WorldgenContent) -> Vec<BlockId> {
    content
        .features
        .iter()
        .filter_map(|feature| match feature {
            Feature::Tree(tree) => Some(tree.leaves),
            _ => None,
        })
        .collect()
}

fn place_rooted_in(
    generator: &Generator,
    (source, ravines): (ColumnPos, &Nearby),
    column: &mut Column,
    leaves: &[BlockId],
) {
    let content = generator.content();
    let mut rng = Rng::for_column(generator.seed(), source, SURFACE_SALT);
    for _ in 0..ATTEMPTS {
        let x = source.x * SECTION_SIZE + rng.range(0, 15);
        let z = source.z * SECTION_SIZE + rng.range(0, 15);
        let mut roll = rng.unit() * f64::from(ATTEMPTS);
        let variant = rng.next_u64();
        if !reaches(column.pos, x, z) {
            continue;
        }
        let (sample, biome) = generator.sample(x, z);
        if sample.height < content.terrain.sea_level {
            continue;
        }
        let mut rooted = biome
            .features
            .iter()
            .filter(|&&(f, _)| !matches!(content.features[f], Feature::Plants(_)));
        let Some(&(feature, _)) = rooted.find(|&&(_, per_column)| {
            roll -= per_column;
            roll < 0.0
        }) else {
            continue;
        };
        let height = generator.surface_height_at(&sample, x, z);
        let feature = &content.features[feature];
        let over_cave = generator.tunnel_may_open(x, z, height..=height)
            || near_ravine(feature, variant, (x, z), ravines);
        if height < content.terrain.sea_level || over_cave {
            continue;
        }
        let ground = generator.ground_block(x, z, &sample, biome, height);
        let mut writer = Writer {
            column: &mut *column,
            x,
            z,
            base: height + 1,
            leaves,
        };
        place(feature, ground, variant, &mut writer);
    }
}

/// Whether any block of the feature would stand within `CLEARANCE` of a ravine open at
/// the surface: no tree leaning over the void.
fn near_ravine(feature: &Feature, variant: u64, (x, z): (i32, i32), ravines: &Nearby) -> bool {
    if !ravines.surface_reaches(x, z, MAX_MARGIN) {
        return false;
    }
    let spread = match feature {
        Feature::Tree(tree) => tree
            .parts(variant)
            .iter()
            .map(|part| match part {
                Part::Leaf((dx, _, dz))
                | Part::Log((dx, _, dz))
                | Part::Vine {
                    top: (dx, _, dz), ..
                } => f64::from(*dx).hypot(f64::from(*dz)),
            })
            .fold(0.0, f64::max),
        _ => 0.0,
    };
    ravines.surface_reaches(x, z, CLEARANCE + spread)
}

/// Whether a feature rooted at `(x, z)` may write into the column at `pos`.
fn reaches(pos: ColumnPos, x: i32, z: i32) -> bool {
    let reach = MAX_CROWN as i32 + 1;
    let near = |c: i32, origin: i32| {
        let start = origin * SECTION_SIZE;
        (start - reach..start + SECTION_SIZE + reach).contains(&c)
    };
    near(x, pos.x) && near(z, pos.z)
}

/// Writes the blocks of one feature that fall inside the column being generated.
struct Writer<'a> {
    column: &'a mut Column,
    x: i32,
    z: i32,
    base: i32,
    leaves: &'a [BlockId],
}

impl Writer<'_> {
    /// Local coordinates of `(dx, dz)` from the origin, if inside the column.
    fn local(&self, dx: i32, dz: i32) -> Option<(u32, u32)> {
        let local = |c: i32, origin: i32| c - origin * SECTION_SIZE;
        let lx = local(self.x + dx, self.column.pos.x);
        let lz = local(self.z + dz, self.column.pos.z);
        ((0..16).contains(&lx) && (0..16).contains(&lz)).then_some((lx as u32, lz as u32))
    }

    /// `(dx, dy, dz)` from the feature origin. Only replaces air (or `over`).
    fn put(&mut self, offset: (i32, i32, i32), block: BlockId, over: Option<BlockId>) {
        self.put_over(offset, block, &|current| Some(current) == over);
    }

    /// A log replaces air and the leaves of any tree.
    fn put_log(&mut self, offset: (i32, i32, i32), log: BlockId) {
        let leaves = self.leaves;
        self.put_over(offset, log, &|current| leaves.contains(&current));
    }

    fn put_over(
        &mut self,
        offset: (i32, i32, i32),
        block: BlockId,
        over: &dyn Fn(BlockId) -> bool,
    ) {
        let Some((lx, lz)) = self.local(offset.0, offset.2) else {
            return;
        };
        let y = self.base + offset.1;
        let current = self.column.get(lx, y, lz);
        if current == BlockId::AIR || over(current) {
            self.column.set(lx, y, lz, block);
        }
    }

    /// A run of `block` down from `top`, stopping at the first block that is not air.
    fn hang(&mut self, top: (i32, i32, i32), length: i32, block: BlockId) {
        let Some((lx, lz)) = self.local(top.0, top.2) else {
            return;
        };
        let top = self.base + top.1;
        for y in (top - length + 1..=top).rev() {
            if self.column.get(lx, y, lz) != BlockId::AIR {
                return;
            }
            self.column.set(lx, y, lz, block);
        }
    }
}

fn place(feature: &Feature, ground: BlockId, variant: u64, writer: &mut Writer<'_>) {
    match feature {
        Feature::Tree(tree) if tree.on.contains(&ground) => place_tree(tree, variant, writer),
        Feature::Pillar { block, height, on } if on.contains(&ground) => {
            let (low, high) = *height;
            let top = low as i32 + (variant % u64::from(high - low + 1)) as i32;
            for y in 0..top {
                writer.put((0, y, 0), *block, None);
            }
        }
        _ => {}
    }
}

fn place_tree(tree: &Tree, variant: u64, writer: &mut Writer<'_>) {
    for part in tree.parts(variant) {
        match (part, tree.vines) {
            // Leaves win over the vines of a neighbouring tree of the same kind.
            (Part::Leaf(offset), _) => writer.put(offset, tree.leaves, tree.vines),
            (Part::Log(offset), _) => writer.put_log(offset, tree.log),
            (Part::Vine { top, length }, Some(vine)) => writer.hang(top, length, vine),
            (Part::Vine { .. }, None) => {}
        }
    }
}
