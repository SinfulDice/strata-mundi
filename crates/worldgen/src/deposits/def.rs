use super::WOBBLE;
use crate::content::Resolver;
use crate::defs::FeatureDef;
use crate::fnv1a;
use strata_core::content::BlockId;
use strata_core::coords::WORLD_HEIGHT;
use strata_core::id::Identifier;
use strata_core::pack::PackError;

/// A `Deposit` feature with blocks resolved and sizes bounded.
#[derive(Debug, Clone)]
pub struct Deposit {
    pub ore: BlockId,
    pub deep_ore: BlockId,
    pub filler: BlockId,
    pub deep_filler: BlockId,
    pub ore_share: f64,
    pub length: (u32, u32),
    pub thickness: (u32, u32),
    pub min_y: i32,
    pub max_y: i32,
    pub cell: i32,
    pub chance: f64,
    pub sign: Option<BlockId>,
    pub sign_share: f64,
    pub sign_on: Vec<BlockId>,
    /// Hash of the feature's identifier: veins stay put when other features are added.
    pub salt: u64,
}

const MAX_LENGTH: u32 = 128;
const MAX_THICKNESS: u32 = 24;

impl Deposit {
    /// `None` if `def` is not a deposit or names an unknown block (the error is recorded).
    /// Bounded so that pack data can't make generation absurdly slow.
    pub(crate) fn resolve(id: &Identifier, def: &FeatureDef, r: &mut Resolver<'_>) -> Option<Self> {
        let FeatureDef::Deposit {
            ore,
            deep_ore,
            filler,
            deep_filler,
            ore_share,
            length,
            thickness,
            min_y,
            max_y,
            cell,
            chance,
            sign,
            sign_share,
            sign_on,
        } = def
        else {
            return None;
        };
        let errors = r.errors.len();
        let (ore, filler) = (r.block(ore), r.block(filler));
        let low = (*min_y).clamp(1, WORLD_HEIGHT - 1);
        let high = (*max_y).clamp(1, WORLD_HEIGHT - 1);
        let deposit = Self {
            ore,
            deep_ore: deep_ore.as_ref().map_or(ore, |id| r.block(id)),
            filler,
            deep_filler: deep_filler.as_ref().map_or(filler, |id| r.block(id)),
            ore_share: ore_share.clamp(0.0, 1.0),
            length: bounded(*length, 8, MAX_LENGTH),
            thickness: bounded(*thickness, 2, MAX_THICKNESS),
            min_y: low.min(high),
            max_y: low.max(high),
            cell: (*cell).clamp(64, 4096) as i32,
            chance: chance.clamp(0.0, 1.0),
            sign: sign.as_ref().map(|id| r.block(id)),
            sign_share: sign_share.clamp(0.0, 0.5),
            sign_on: r.blocks(sign_on),
            salt: fnv1a(id.to_string().as_bytes()),
        };
        if r.errors.len() > errors {
            let message = format!("deposit `{id}` ignored: unknown block");
            r.errors.push(PackError::invalid(r.source, message));
            return None;
        }
        Some(deposit)
    }

    /// Farthest a block of a vein may lie from its cell's center point, horizontally.
    pub(super) fn reach(&self) -> i32 {
        let radius = f64::from(self.thickness.1) / 2.0 * (1.0 + WOBBLE);
        (f64::from(self.length.1) / 2.0 + radius).ceil() as i32 + 1
    }
}

fn bounded((a, b): (u32, u32), min: u32, max: u32) -> (u32, u32) {
    (a.min(b).clamp(min, max), a.max(b).clamp(min, max))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::path::Path;
    use strata_core::content::{Block, Registry};

    fn def(block: &str, min_y: i32, max_y: i32) -> FeatureDef {
        let id = |s: &str| s.parse::<Identifier>().unwrap();
        FeatureDef::Deposit {
            ore: id(block),
            deep_ore: None,
            filler: id("test:rock"),
            deep_filler: None,
            ore_share: 3.0,
            length: (500, 1),
            thickness: (0, 90),
            min_y,
            max_y,
            cell: 1,
            chance: -1.0,
            sign: Some(id("test:rock")),
            sign_share: 0.9,
            sign_on: vec![id("test:rock")],
        }
    }

    fn resolve(def: &FeatureDef) -> (Option<Deposit>, usize) {
        let mut blocks = Registry::<BlockId, Block>::default();
        for name in ["test:air", "test:ore", "test:rock"] {
            blocks.insert(name.parse().unwrap(), Block::air());
        }
        let mut errors = Vec::new();
        let mut r = Resolver {
            blocks: &blocks,
            errors: &mut errors,
            source: Path::new("test.ron"),
        };
        let deposit = Deposit::resolve(&"test:deposit".parse().unwrap(), def, &mut r);
        (deposit, errors.len())
    }

    #[test]
    fn out_of_range_values_are_bounded() {
        let (deposit, errors) = resolve(&def("test:ore", 9000, -5));
        let d = deposit.unwrap();
        assert_eq!(errors, 0);
        assert_eq!((d.min_y, d.max_y), (1, WORLD_HEIGHT - 1));
        assert_eq!(d.length, (8, MAX_LENGTH));
        assert_eq!(d.thickness, (2, MAX_THICKNESS));
        assert_eq!(
            (d.cell, d.chance, d.ore_share, d.sign_share),
            (64, 0.0, 1.0, 0.5)
        );
        assert_eq!(d.deep_ore, d.ore);
    }

    #[test]
    fn swapped_heights_are_reordered() {
        let d = resolve(&def("test:ore", 120, 40)).0.unwrap();
        assert_eq!((d.min_y, d.max_y), (40, 120));
    }

    #[test]
    fn unknown_block_drops_the_deposit() {
        let (deposit, errors) = resolve(&def("test:missing", 10, 20));
        assert!(deposit.is_none());
        assert!(errors >= 1);
    }
}
