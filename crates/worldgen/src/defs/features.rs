//! `features/<name>.ron`.

use serde::Deserialize;
use strata_core::id::Identifier;

/// `features/<name>.ron`.
#[derive(Debug, Clone, Deserialize)]
pub enum FeatureDef {
    Tree {
        log: Identifier,
        leaves: Identifier,
        height: (u32, u32),
        crown: Crown,
        #[serde(default)]
        trunk: Trunk,
        /// Block hanging under the crown's lowest outer leaves; `None` = no vines.
        #[serde(default)]
        vines: Option<Identifier>,
        on: Vec<Identifier>,
    },
    /// A single stack of one block (cactus).
    Pillar {
        block: Identifier,
        height: (u32, u32),
        on: Vec<Identifier>,
    },
    /// Patches of small plants on the ground: `count` plants within `radius` blocks of
    /// the patch center, each picked from `blocks` by weight, only on `on` and in the open
    /// air (never under water). A biome's `per_column` counts patches.
    Plants {
        blocks: Vec<(Identifier, u32)>,
        on: Vec<Identifier>,
        radius: u32,
        count: (u32, u32),
    },
    /// Small ore veins inside stone; `deep_block` replaces deep stone.
    Ore {
        block: Identifier,
        deep_block: Option<Identifier>,
        vein_size: u32,
        per_column: f64,
        min_y: i32,
        max_y: i32,
    },
    /// A giant vein of `filler` rock rich in ore, at most one per `cell`×`cell` blocks.
    Deposit {
        ore: Identifier,
        /// Replaces `ore` in deep stone.
        #[serde(default)]
        deep_ore: Option<Identifier>,
        filler: Identifier,
        /// Replaces `filler` in deep stone.
        #[serde(default)]
        deep_filler: Option<Identifier>,
        /// Share of the vein's blocks that are ore, the rest being filler.
        ore_share: f64,
        /// Smallest and largest length, in blocks.
        length: (u32, u32),
        /// Smallest and largest thickness at the middle, in blocks.
        thickness: (u32, u32),
        /// Height range of the vein's middle.
        min_y: i32,
        max_y: i32,
        /// Side of the square cells of the world, in blocks; each may hold one vein.
        cell: u32,
        /// Share of the cells holding a vein.
        chance: f64,
        /// Block hinting at the vein on the ground above it; `None` = no hint.
        #[serde(default)]
        sign: Option<Identifier>,
        /// Share of the ground blocks above the vein replaced by `sign`.
        #[serde(default)]
        sign_share: f64,
        /// Ground blocks `sign` may replace.
        #[serde(default)]
        sign_on: Vec<Identifier>,
    },
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Crown {
    /// Rounded oak-like crown.
    Round { radius: u32 },
    /// Layered cone (fir).
    Cone { radius: u32 },
    /// Ellipsoid around the trunk's top: `radius` across, `height` up and down.
    Blob { radius: u32, height: u32 },
    /// Wide, flat canopy of two layers (acacia).
    Flat { radius: u32 },
    /// Giant mushroom cap: flat if `skirt` is 0, else a dome whose sides hang down
    /// `skirt` blocks.
    Cap { radius: u32, skirt: u32 },
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub enum Trunk {
    /// One block wide.
    #[default]
    Straight,
    /// Two blocks by two (jungle).
    Thick,
    /// Straight, then leaning toward a random side by `lean` blocks (acacia).
    Leaning { lean: (u32, u32) },
}
