#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::collections::BTreeSet;
use strata_core::coords::{ColumnPos, SECTION_SIZE};
use strata_core::lang::LANGUAGES;

fn positions() -> Vec<ColumnPos> {
    [(0, 0), (1, 0), (-3, 7), (40, -12), (-1, -1)]
        .map(|(x, z)| ColumnPos { x, z })
        .to_vec()
}

#[test]
fn same_seed_gives_same_world_in_any_order() {
    let forward = common::generator(42);
    let backward = common::generator(42);
    let first: Vec<_> = positions()
        .into_iter()
        .map(|p| forward.generate(p))
        .collect();
    let mut second: Vec<_> = positions()
        .into_iter()
        .rev()
        .map(|p| backward.generate(p))
        .collect();
    second.reverse();
    assert_eq!(first, second);
}

#[test]
fn different_seeds_give_different_worlds() {
    let pos = ColumnPos { x: 3, z: 3 };
    assert_ne!(
        common::generator(1).generate(pos),
        common::generator(2).generate(pos)
    );
}

#[test]
fn every_biome_appears_and_is_translated() {
    let (content, worldgen) = common::load();
    let generator = strata_worldgen::Generator::new(strata_worldgen::Seed(7), worldgen.clone());
    let mut found = BTreeSet::new();
    for gx in -60..60 {
        for gz in -60..60 {
            found.insert(generator.sample(gx * 128, gz * 128).1.id.to_string());
        }
    }
    let all: BTreeSet<String> = worldgen.biomes().iter().map(|b| b.id.to_string()).collect();
    assert_eq!(found, all);
    for biome in worldgen.biomes() {
        for language in LANGUAGES {
            assert!(
                content
                    .translations
                    .has(language, &biome.id.translation_key("biome"))
            );
        }
    }
}

#[test]
fn terrain_stays_in_the_height_range_with_bedrock_and_sea() {
    let generator = common::generator(5);
    let sea_level = generator.content().terrain.sea_level;
    let bedrock = generator.content().terrain.bedrock;
    let liquid = [
        generator.content().terrain.water,
        generator.content().terrain.ice,
    ];
    let mut saw_water = false;
    for gx in -40..40 {
        let (sample, _) = generator.sample(gx * 97, gx * 61);
        assert!((1..420).contains(&sample.height));
        if sample.height < sea_level {
            saw_water = true;
            let pos = ColumnPos {
                x: (gx * 97).div_euclid(SECTION_SIZE),
                z: (gx * 61).div_euclid(SECTION_SIZE),
            };
            let column = generator.generate(pos);
            let (lx, lz) = (
                (gx * 97).rem_euclid(16) as u32,
                (gx * 61).rem_euclid(16) as u32,
            );
            assert_eq!(column.get(lx, 0, lz), bedrock);
            assert!(liquid.contains(&column.get(lx, sea_level, lz)));
        }
    }
    assert!(saw_water, "some sampled points should be under the sea");
}

#[test]
fn spawn_is_welcoming() {
    let generator = common::generator(1234);
    let spawn = generator.spawn_point();
    let (sample, biome) = generator.sample(spawn.x, spawn.z);
    assert!(biome.spawn, "spawned in {}", biome.id);
    assert!(sample.height > generator.content().terrain.sea_level);
    assert_eq!(spawn.y, sample.height + 1);
}

/// Trees rooted near a column border must be whole on both sides, whatever the order and
/// the thread each column was generated on.
#[test]
fn trees_cross_column_borders_whole() {
    let generator = std::sync::Arc::new(common::generator(3));
    let (_, worldgen) = common::load();
    let trees: Vec<_> = worldgen
        .features
        .iter()
        .filter_map(|f| match f {
            strata_worldgen::Feature::Tree { log, leaves, .. } => Some((*log, *leaves)),
            _ => None,
        })
        .collect();
    let forests = (0..2000)
        .map(|i| ((i % 40) * 64 - 1280, (i / 40) * 64 - 1600))
        .filter(|&(x, z)| generator.sample(x, z).1.id.name() == "forest")
        .map(|(x, z)| ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        });
    let checked: usize = forests
        .take(6)
        .map(|center| check_border_trees(&generator, center, &trees))
        .sum();
    assert!(checked > 0, "no tree top found on forest column borders");
}

/// Generates the 3×3 columns around `center` on separate threads, in reverse order, and
/// checks every tree top on the border of `center` has leaves on its four sides.
fn check_border_trees(
    generator: &std::sync::Arc<strata_worldgen::Generator>,
    center: ColumnPos,
    trees: &[(strata_core::content::BlockId, strata_core::content::BlockId)],
) -> usize {
    let positions: Vec<ColumnPos> = (-1..=1)
        .flat_map(|dz| {
            (-1..=1).map(move |dx| ColumnPos {
                x: center.x + dx,
                z: center.z + dz,
            })
        })
        .collect();
    let columns: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = positions
            .iter()
            .rev()
            .map(|&p| scope.spawn(move || generator.generate(p)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let block = |x: i32, y: i32, z: i32| {
        let pos = ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        };
        columns
            .iter()
            .find(|c| c.pos == pos)
            .map(|c| c.get(x.rem_euclid(16) as u32, y, z.rem_euclid(16) as u32))
    };
    let mut checked = 0;
    for lx in 0..16 {
        for lz in 0..16 {
            if !(lx == 0 || lx == 15 || lz == 0 || lz == 15) {
                continue;
            }
            let (x, z) = (center.x * 16 + lx, center.z * 16 + lz);
            for y in 100..320 {
                let here = block(x, y, z);
                let Some(&(log, leaves)) = trees.iter().find(|(l, _)| Some(*l) == here) else {
                    continue;
                };
                if block(x, y + 1, z) == Some(log) {
                    continue;
                }
                for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let side = block(x + dx, y, z + dz);
                    assert!(
                        side == Some(leaves) || trees.iter().any(|(l, _)| side == Some(*l)),
                        "tree top at {x},{y},{z} lacks leaves"
                    );
                }
                checked += 1;
            }
        }
    }
    checked
}
