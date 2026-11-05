//! Tree shapes: a trunk and a crown, computed as a pure list of parts from a variant, so
//! every column writing its share of a tree agrees on the whole tree.

use crate::content::{MAX_CROWN, Resolver, bounded};
use crate::defs::{Crown, Trunk};
use crate::rng::Rng;
use std::collections::HashSet;
use strata_core::content::BlockId;
use strata_core::id::Identifier;

const MAX_TREE_HEIGHT: u32 = 32;
const MAX_CROWN_HEIGHT: u32 = 8;
const MAX_SKIRT: u32 = 4;
const MAX_VINE: u32 = 4;
/// Share of the lowest outer leaves that grow a vine.
const VINE_CHANCE: f64 = 0.3;

#[derive(Debug, Clone)]
pub struct Tree {
    pub log: BlockId,
    pub leaves: BlockId,
    pub height: (u32, u32),
    pub crown: Crown,
    pub trunk: Trunk,
    pub vines: Option<BlockId>,
    pub on: Vec<BlockId>,
}

type Offset = (i32, i32, i32);

/// One piece of a tree, `(dx, dy, dz)` from the block above the ground it grows on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Leaf((i32, i32, i32)),
    /// Replaces leaves.
    Log((i32, i32, i32)),
    /// Hangs down from `top` over `length` blocks, stopping at the first non-air block.
    Vine {
        top: (i32, i32, i32),
        length: i32,
    },
}

/// The fields of `FeatureDef::Tree`.
pub struct TreeDef<'a> {
    pub log: &'a Identifier,
    pub leaves: &'a Identifier,
    pub height: (u32, u32),
    pub crown: Crown,
    pub trunk: Trunk,
    pub vines: Option<&'a Identifier>,
    pub on: &'a [Identifier],
}

impl Tree {
    /// Bounded so that the whole tree stays within `MAX_CROWN` blocks of its root.
    pub(crate) fn resolve(def: TreeDef<'_>, r: &mut Resolver<'_>) -> Self {
        let wide = u32::from(matches!(def.trunk, Trunk::Thick));
        let crown = bounded_crown(def.crown, MAX_CROWN - wide);
        let trunk = match def.trunk {
            Trunk::Leaning { lean } => Trunk::Leaning {
                lean: bounded(lean, MAX_CROWN - crown.radius()),
            },
            other => other,
        };
        Self {
            log: r.block(def.log),
            leaves: r.block(def.leaves),
            height: bounded(def.height, MAX_TREE_HEIGHT),
            crown,
            trunk,
            vines: def.vines.map(|id| r.block(id)),
            on: r.blocks(def.on),
        }
    }

    /// Leaves first, then logs over them, then vines into what is left of the air.
    pub fn parts(&self, variant: u64) -> Vec<Part> {
        let mut rng = Rng::from_state(variant);
        let height = pick(&mut rng, self.height).max(1);
        let (logs, top) = trunk_logs(self.trunk, height, &mut rng);
        let mut leaves = Vec::new();
        crown_shape(self.crown, height, &mut rng, |(dx, dy, dz)| {
            leaves.push((dx + top.0, dy, dz + top.1));
        });
        let vines = match self.vines {
            Some(_) => hanging_vines(&leaves, &mut rng),
            None => Vec::new(),
        };
        let leaves = leaves.into_iter().map(Part::Leaf);
        leaves
            .chain(logs.into_iter().map(Part::Log))
            .chain(vines)
            .collect()
    }
}

impl Crown {
    fn radius(self) -> u32 {
        match self {
            Self::Round { radius }
            | Self::Cone { radius }
            | Self::Blob { radius, .. }
            | Self::Flat { radius }
            | Self::Cap { radius, .. } => radius,
        }
    }
}

fn bounded_crown(crown: Crown, max: u32) -> Crown {
    match crown {
        Crown::Round { radius } => Crown::Round {
            radius: radius.min(max),
        },
        Crown::Cone { radius } => Crown::Cone {
            radius: radius.min(max),
        },
        Crown::Blob { radius, height } => Crown::Blob {
            radius: radius.clamp(1, max),
            height: height.clamp(1, MAX_CROWN_HEIGHT),
        },
        Crown::Flat { radius } => Crown::Flat {
            radius: radius.clamp(2, max),
        },
        Crown::Cap { radius, skirt } => Crown::Cap {
            radius: radius.clamp(1, max),
            skirt: skirt.min(MAX_SKIRT),
        },
    }
}

fn pick(rng: &mut Rng, (low, high): (u32, u32)) -> i32 {
    rng.range(low as i32, high as i32)
}

/// The trunk's blocks and the horizontal offset of its top.
fn trunk_logs(trunk: Trunk, height: i32, rng: &mut Rng) -> (Vec<Offset>, (i32, i32)) {
    match trunk {
        Trunk::Straight => ((0..height).map(|y| (0, y, 0)).collect(), (0, 0)),
        Trunk::Thick => {
            let logs = (0..height)
                .flat_map(|y| [(0, y, 0), (1, y, 0), (0, y, 1), (1, y, 1)])
                .collect();
            (logs, (0, 0))
        }
        Trunk::Leaning { lean } => {
            let steps = pick(rng, lean).min(height - 1).max(0);
            let (sx, sz) = LEAN_DIRECTIONS[rng.range(0, 7) as usize];
            let straight = height - steps;
            let logs = (0..height)
                .map(|y| {
                    let shift = (y - straight + 1).max(0);
                    (sx * shift, y, sz * shift)
                })
                .collect();
            (logs, (sx * steps, sz * steps))
        }
    }
}

const LEAN_DIRECTIONS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// Every crown covers the block above the trunk's top and its four sides.
fn crown_shape(crown: Crown, trunk: i32, rng: &mut Rng, mut leaf: impl FnMut(Offset)) {
    match crown {
        Crown::Round { radius } => round(radius as i32, trunk, rng, leaf),
        Crown::Cone { radius } => cone(radius as i32, trunk, leaf),
        Crown::Blob { radius, height } => blob(radius as i32, height as i32, trunk, rng, leaf),
        Crown::Flat { radius } => {
            disk(radius as i32, trunk - 1, rng, &mut leaf);
            disk((radius as i32 - 2).max(1), trunk, rng, &mut leaf);
        }
        Crown::Cap { radius, skirt } => cap(radius as i32, skirt as i32, trunk, leaf),
    }
}

/// Oak-like: two wide layers around the trunk's top, two narrower ones above.
fn round(r: i32, trunk: i32, rng: &mut Rng, mut leaf: impl FnMut(Offset)) {
    for dy in -2..=1 {
        let layer_r = if dy >= 0 { r - 1 } else { r };
        for dx in -layer_r..=layer_r {
            for dz in -layer_r..=layer_r {
                let corner = dx.abs() == layer_r && dz.abs() == layer_r;
                let trimmed = corner && rng.unit() < 0.5;
                if !trimmed || dy < -1 {
                    leaf((dx, trunk + dy, dz));
                }
            }
        }
    }
    leaf((0, trunk + 1, 0));
}

/// Fir-like: layers shrinking toward the top, alternating wide and narrow.
fn cone(r: i32, trunk: i32, mut leaf: impl FnMut(Offset)) {
    for dy in 2..=trunk {
        let from_top = trunk - dy;
        let layer_r = ((from_top + 1) / 2).min(r) - i32::from(from_top % 2 == 1 && from_top > 2);
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

/// An ellipsoid centered on the trunk's top, its outer shell thinned at random.
fn blob(r: i32, h: i32, trunk: i32, rng: &mut Rng, mut leaf: impl FnMut(Offset)) {
    let (rr, hh) = (f64::from(r) + 0.5, f64::from(h) + 0.5);
    for dy in -h..=h {
        for dx in -r..=r {
            for dz in -r..=r {
                let d = f64::from(dx * dx + dz * dz) / (rr * rr) + f64::from(dy * dy) / (hh * hh);
                let core = dx.abs() + dz.abs() <= 1 && (0..=1).contains(&dy);
                if core || (d <= 1.0 && (d < 0.6 || rng.unit() < 0.7)) {
                    leaf((dx, trunk - 1 + dy, dz));
                }
            }
        }
    }
}

/// A flat round layer with ragged edges.
fn disk(r: i32, y: i32, rng: &mut Rng, leaf: &mut impl FnMut(Offset)) {
    let edge = f64::from(r) + 0.5;
    for dx in -r..=r {
        for dz in -r..=r {
            let d = f64::from(dx * dx + dz * dz).sqrt();
            let ragged = d > edge - 1.0 && rng.unit() < 0.35;
            if d <= edge && (!ragged || dx.abs() + dz.abs() <= 1) {
                leaf((dx, y, dz));
            }
        }
    }
}

/// Giant mushroom cap on top of the stem: flat without a skirt, a dome with one.
fn cap(r: i32, skirt: i32, trunk: i32, mut leaf: impl FnMut(Offset)) {
    let top = if skirt > 0 { r - 1 } else { r };
    for dx in -top..=top {
        for dz in -top..=top {
            if skirt > 0 || dx.abs() != top || dz.abs() != top {
                leaf((dx, trunk, dz));
            }
        }
    }
    for dy in 1..=skirt {
        for dx in -r..=r {
            for dz in -r..=r {
                let ring = dx.abs().max(dz.abs()) == r;
                if ring && (dx.abs() != r || dz.abs() != r) {
                    leaf((dx, trunk - dy, dz));
                }
            }
        }
    }
}

/// Vines under some of the lowest leaves on the crown's outer edge.
fn hanging_vines(leaves: &[Offset], rng: &mut Rng) -> Vec<Part> {
    let set: HashSet<_> = leaves.iter().copied().collect();
    let outer = |&(x, y, z): &Offset| {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|(dx, dz)| !set.contains(&(x + dx, y, z + dz)))
    };
    let mut vines = Vec::new();
    for &(x, y, z) in leaves {
        let lowest = !set.contains(&(x, y - 1, z));
        if lowest && outer(&(x, y, z)) && rng.unit() < VINE_CHANCE {
            let length = rng.range(1, MAX_VINE as i32);
            vines.push(Part::Vine {
                top: (x, y - 1, z),
                length,
            });
        }
    }
    vines
}
