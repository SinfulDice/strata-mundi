#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;
use strata_worldgen::defs::Crown;
use strata_worldgen::{Feature, Generator, MAX_CROWN, Part, Seed, Tree, WorldgenContent};

fn trees(worldgen: &WorldgenContent) -> Vec<Tree> {
    worldgen
        .features
        .iter()
        .filter_map(|f| match f {
            Feature::Tree(tree) => Some(tree.clone()),
            _ => None,
        })
        .collect()
}

/// Trees of this milestone: new crowns or trunks (bushes of one log excepted).
fn new_trees(worldgen: &WorldgenContent) -> Vec<Tree> {
    trees(worldgen)
        .into_iter()
        .filter(|t| !matches!(t.crown, Crown::Round { .. } | Crown::Cone { .. }))
        .filter(|t| t.height.1 > 1)
        .collect()
}

fn offset(part: &Part) -> (i32, i32, i32) {
    match *part {
        Part::Leaf(o) | Part::Log(o) => o,
        Part::Vine { top, .. } => top,
    }
}

#[test]
fn every_tree_shape_stays_within_its_bounds() {
    let (_, worldgen) = common::load();
    let reach = MAX_CROWN as i32;
    assert!(new_trees(&worldgen).len() >= 5, "new tree shapes missing");
    for tree in trees(&worldgen) {
        for variant in (0..400u64).map(|v| v.wrapping_mul(0x9E37_79B9_7F4A_7C15)) {
            let parts = tree.parts(variant);
            let logs = parts.iter().filter(|p| matches!(p, Part::Log(_))).count();
            assert!(logs > 0, "{tree:?} has no trunk");
            for part in &parts {
                let (dx, dy, dz) = offset(part);
                assert!(dx.abs() <= reach && dz.abs() <= reach, "{tree:?}: {part:?}");
                assert!((-2..=48).contains(&dy), "{tree:?}: {part:?}");
                if let Part::Vine { length, .. } = part {
                    assert!((1..=8).contains(length));
                }
            }
        }
    }
}

#[test]
fn trees_are_the_same_for_the_same_variant() {
    let (_, worldgen) = common::load();
    for tree in trees(&worldgen) {
        assert_eq!(tree.parts(12345), tree.parts(12345));
    }
}

/// Each new tree replaces every tree of the world in turn, so forests are full of it; the
/// trees rooted near a column border must be whole on both sides, whatever the order.
#[test]
fn new_trees_cross_column_borders_whole() {
    let (_, worldgen) = common::load();
    for tree in new_trees(&worldgen) {
        let mut forced = (*worldgen).clone();
        for feature in &mut forced.features {
            if matches!(feature, Feature::Tree(_)) {
                *feature = Feature::Tree(tree.clone());
            }
        }
        let generator = Generator::new(Seed(3), Arc::new(forced));
        let checked: usize = forest_columns(&generator)
            .take(3)
            .map(|center| check_border_tops(&generator, center, &tree))
            .sum();
        assert!(checked > 0, "no top of {tree:?} on a forest column border");
    }
}

fn forest_columns(generator: &Generator) -> impl Iterator<Item = ColumnPos> + '_ {
    (0..2000)
        .map(|i| ((i % 40) * 64 - 1280, (i / 40) * 64 - 1600))
        .filter(|&(x, z)| generator.sample(x, z).1.id.name() == "forest")
        .map(|(x, z)| ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        })
}

fn neighbourhood(generator: &Generator, center: ColumnPos) -> Vec<Column> {
    let positions: Vec<ColumnPos> = (-1..=1)
        .flat_map(|dz| {
            (-1..=1).map(move |dx| ColumnPos {
                x: center.x + dx,
                z: center.z + dz,
            })
        })
        .collect();
    std::thread::scope(|scope| {
        let handles: Vec<_> = positions
            .iter()
            .rev()
            .map(|&p| scope.spawn(move || generator.generate(p)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

/// Every trunk top on the border of `center` (a log with no log on the 3×3 above) must
/// carry leaves above it and on the four sides of those.
fn check_border_tops(generator: &Generator, center: ColumnPos, tree: &Tree) -> usize {
    let columns = neighbourhood(generator, center);
    let block = |x: i32, y: i32, z: i32| -> Option<BlockId> {
        let pos = ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        };
        let column = columns.iter().find(|c| c.pos == pos)?;
        Some(column.get(x.rem_euclid(16) as u32, y, z.rem_euclid(16) as u32))
    };
    let canopy = |b: Option<BlockId>| b == Some(tree.leaves) || b == Some(tree.log);
    let mut checked = 0;
    for (lx, lz) in border_cells() {
        let (x, z) = (center.x * 16 + lx, center.z * 16 + lz);
        for y in 100..330 {
            if block(x, y, z) != Some(tree.log) {
                continue;
            }
            let log_above = (-1..=1)
                .flat_map(|dx| (-1..=1).map(move |dz| (dx, dz)))
                .any(|(dx, dz)| block(x + dx, y + 1, z + dz) == Some(tree.log));
            if log_above {
                continue;
            }
            assert!(canopy(block(x, y + 1, z)), "top at {x},{y},{z} uncovered");
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let side = block(x + dx, y + 1, z + dz);
                assert!(
                    canopy(side),
                    "top at {x},{y},{z} lacks a side of its crown: {side:?} {:?}",
                    tree.crown
                );
            }
            checked += 1;
        }
    }
    checked
}

fn border_cells() -> impl Iterator<Item = (i32, i32)> {
    (0..16)
        .flat_map(|lx| (0..16).map(move |lz| (lx, lz)))
        .filter(|&(lx, lz)| lx == 0 || lx == 15 || lz == 0 || lz == 15)
}
