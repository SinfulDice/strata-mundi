#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use strata_core::content::BlockId;
use strata_core::coords::{ColumnPos, WORLD_HEIGHT};
use strata_core::generation::{FlatLayer, FlatSettings, WorldgenSettings};
use strata_core::id::Identifier;
use strata_worldgen::{Generator, Seed, SettingsError, WorldgenContent};

fn id(text: &str) -> Identifier {
    text.parse().expect("valid id")
}

fn of_type(name: &str) -> WorldgenSettings {
    WorldgenSettings {
        world_type: id(&format!("strata:{name}")),
        ..WorldgenSettings::default()
    }
}

fn generator(seed: u64, settings: &WorldgenSettings) -> Generator {
    Generator::with_settings(Seed(seed), &common::load().1, settings).expect("valid settings")
}

fn positions() -> Vec<ColumnPos> {
    [(0, 0), (1, 0), (-3, 7), (12, -5), (40, 40), (-25, 3)]
        .map(|(x, z)| ColumnPos { x, z })
        .to_vec()
}

/// Points of a coarse grid over a wide area, in blocks.
fn grid(half: i32, step: usize) -> impl Iterator<Item = (i32, i32)> {
    (-half..=half)
        .step_by(step)
        .flat_map(move |z| (-half..=half).step_by(step).map(move |x| (x, z)))
}

#[test]
fn pack_world_types_are_listed_in_order() {
    let (_, worldgen) = common::load();
    let names: Vec<&str> = worldgen.world_types().iter().map(|t| t.id.name()).collect();
    assert_eq!(
        names,
        ["normal", "flat", "amplified", "single_biome", "archipelago"]
    );
}

#[test]
fn default_settings_keep_the_world_unchanged() {
    let (_, worldgen) = common::load();
    let before = Generator::new(Seed(42), Arc::clone(&worldgen));
    // Through the whole application, not the shortcut taken for the default settings.
    let changed = worldgen
        .with_settings(&WorldgenSettings::default())
        .expect("normal type");
    let after = Generator::new(Seed(42), Arc::new(changed));
    for pos in positions() {
        assert_eq!(
            before.generate_with_biomes(pos),
            after.generate_with_biomes(pos)
        );
    }
    assert_eq!(before.spawn_point(), after.spawn_point());
}

#[test]
fn every_type_is_deterministic_in_any_order() {
    for name in ["normal", "flat", "amplified", "single_biome", "archipelago"] {
        let settings = of_type(name);
        let (first, second) = (generator(7, &settings), generator(7, &settings));
        let forward: Vec<_> = positions().into_iter().map(|p| first.generate(p)).collect();
        let mut backward: Vec<_> = positions()
            .into_iter()
            .rev()
            .map(|p| second.generate(p))
            .collect();
        backward.reverse();
        assert!(forward == backward, "{name}");
    }
}

#[test]
fn flat_worlds_have_exactly_their_layers() {
    let (content, _) = common::load();
    let layers = [("bedrock", 1), ("stone", 3), ("dirt", 2), ("sand", 1)];
    let settings = WorldgenSettings {
        flat: Some(FlatSettings {
            layers: layers
                .iter()
                .map(|&(block, thickness)| FlatLayer {
                    block: id(&format!("strata:{block}")),
                    thickness,
                })
                .collect(),
            caves: false,
            features: false,
        }),
        ores: 0,
        ..of_type("flat")
    };
    let generator = generator(3, &settings);
    let block = |name: &str| {
        content
            .blocks
            .key(&id(&format!("strata:{name}")))
            .expect("block")
    };
    let expected: Vec<BlockId> = layers
        .iter()
        .flat_map(|&(name, thickness)| std::iter::repeat_n(block(name), usize::from(thickness)))
        .collect();
    for pos in positions() {
        let column = generator.generate(pos);
        for (lx, lz) in [(0, 0), (7, 9), (15, 15)] {
            for y in 0..WORLD_HEIGHT {
                let wanted = expected.get(y as usize).copied().unwrap_or(BlockId::AIR);
                assert_eq!(column.get(lx, y, lz), wanted, "{pos:?} ({lx}, {y}, {lz})");
            }
        }
    }
    let spawn = generator.spawn_point();
    assert_eq!(spawn.y, expected.len() as i32);
}

#[test]
fn single_biome_worlds_have_one_biome() {
    let settings = WorldgenSettings {
        biome: Some(id("strata:desert")),
        ..of_type("single_biome")
    };
    let generator = generator(42, &settings);
    assert!(grid(3000, 50).all(|(x, z)| generator.sample(x, z).1.id.name() == "desert"));
    let sea_level = generator.content().terrain.sea_level;
    assert!(generator.spawn_point().y > sea_level);
    let default = generator_of("single_biome", 42);
    assert!(grid(1000, 100).all(|(x, z)| default.sample(x, z).1.id.name() == "plains"));
}

fn generator_of(name: &str, seed: u64) -> Generator {
    generator(seed, &of_type(name))
}

#[test]
fn archipelagos_are_mostly_sea_with_reachable_islands() {
    for seed in [1, 42, 99] {
        let generator = generator_of("archipelago", seed);
        let sea_level = generator.content().terrain.sea_level;
        let points: Vec<_> = grid(4000, 40).collect();
        let sea = points
            .iter()
            .filter(|&&(x, z)| generator.sample(x, z).0.height < sea_level)
            .count();
        let share = sea as f64 / points.len() as f64;
        assert!(
            (0.6..0.95).contains(&share),
            "seed {seed}: {share:.2} of sea"
        );
        let spawn = generator.spawn_point();
        assert!(
            generator.surface_height(spawn.x, spawn.z) > sea_level,
            "seed {seed}"
        );
        let column = generator.generate(ColumnPos {
            x: spawn.x.div_euclid(16),
            z: spawn.z.div_euclid(16),
        });
        let below = column.get(
            spawn.x.rem_euclid(16) as u32,
            spawn.y - 1,
            spawn.z.rem_euclid(16) as u32,
        );
        assert!(![BlockId::AIR, generator.content().terrain.water].contains(&below));
    }
}

#[test]
fn amplified_worlds_rise_higher_but_stay_inside() {
    let normal = generator_of("normal", 42);
    let amplified = generator_of("amplified", 42);
    let highest = |generator: &Generator| {
        grid(3000, 25)
            .map(|(x, z)| generator.surface_height(x, z))
            .max()
            .unwrap_or(0)
    };
    let (low, high) = (highest(&normal), highest(&amplified));
    assert!(high > low + 60, "normal {low}, amplified {high}");
    assert!(high < WORLD_HEIGHT, "{high}");
    let max_plain = amplified.content().terrain.relief.max_height;
    let overhang = amplified.content().terrain.relief.overhang_blocks;
    assert!(max_plain + overhang.ceil() <= f64::from(WORLD_HEIGHT - 1));
}

#[test]
fn sliders_change_the_world_within_bounds() {
    let (_, worldgen) = common::load();
    let changed = |settings: WorldgenSettings| worldgen.with_settings(&settings).expect("valid");
    let none = changed(WorldgenSettings {
        caves: 0,
        ores: 0,
        ..WorldgenSettings::default()
    });
    assert!(none.terrain.caves.is_none());
    assert!(none.terrain.ores.is_empty() && none.terrain.deposits.is_empty());
    let low = changed(WorldgenSettings {
        sea_level: Some(-30),
        biome_size: 1,
        ..WorldgenSettings::default()
    });
    assert_eq!(low.terrain.sea_level, 48);
    assert_eq!(low.terrain.biome_scale, worldgen.terrain.biome_scale * 0.25);
    let more = changed(WorldgenSettings {
        caves: 400,
        ..WorldgenSettings::default()
    });
    let (base, wide) = (
        worldgen.terrain.caves.as_ref().expect("caves"),
        more.terrain.caves.as_ref().expect("caves"),
    );
    assert!(wide.spaghetti > base.spaghetti && wide.spaghetti <= 0.5);
    let no_ores = Generator::new(Seed(5), Arc::new(none));
    let ores = ore_blocks(&worldgen);
    let column = no_ores.generate(ColumnPos { x: 3, z: 3 });
    assert!(
        (0..WORLD_HEIGHT).all(|y| !ores.contains(&column.get(8, y, 8))),
        "ore left"
    );
}

fn ore_blocks(worldgen: &WorldgenContent) -> Vec<BlockId> {
    worldgen
        .terrain
        .ores
        .iter()
        .filter_map(|&index| match worldgen.features[index] {
            strata_worldgen::Feature::Ore { block, .. } => Some(block),
            _ => None,
        })
        .collect()
}

#[test]
fn unknown_names_are_errors() {
    let (_, worldgen) = common::load();
    let error = |settings: WorldgenSettings| worldgen.with_settings(&settings).err();
    assert_eq!(
        error(of_type("moon")),
        Some(SettingsError::UnknownType(id("strata:moon")))
    );
    let biome = WorldgenSettings {
        biome: Some(id("strata:nowhere")),
        ..of_type("single_biome")
    };
    assert_eq!(
        error(biome),
        Some(SettingsError::UnknownBiome(id("strata:nowhere")))
    );
    let flat = |layers| WorldgenSettings {
        flat: Some(FlatSettings {
            layers,
            caves: false,
            features: true,
        }),
        ..of_type("flat")
    };
    let cheese = FlatLayer {
        block: id("strata:cheese"),
        thickness: 2,
    };
    assert_eq!(
        error(flat(vec![cheese])),
        Some(SettingsError::UnknownBlock(id("strata:cheese")))
    );
    assert_eq!(error(flat(Vec::new())), Some(SettingsError::NoLayers));
}

#[test]
fn saved_settings_drop_what_their_type_does_not_use() {
    let (_, worldgen) = common::load();
    let stray = WorldgenSettings {
        biome: Some(id("strata:desert")),
        flat: Some(FlatSettings {
            layers: vec![FlatLayer {
                block: id("strata:sand"),
                thickness: 2,
            }],
            caves: false,
            features: false,
        }),
        ..of_type("normal")
    };
    let used = worldgen.used_settings(&stray).expect("known type");
    assert_eq!(used, of_type("normal"));
    let single = WorldgenSettings {
        biome: Some(id("strata:desert")),
        ..of_type("single_biome")
    };
    assert_eq!(worldgen.used_settings(&single), Ok(single.clone()));
}
