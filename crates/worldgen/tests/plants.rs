#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;
use strata_worldgen::{Feature, Generator, Plants, Seed, WorldgenContent};

const SEEDS: [u64; 3] = [42, 7, 2024];

fn plant_features(worldgen: &WorldgenContent) -> Vec<Plants> {
    worldgen
        .features
        .iter()
        .filter_map(|f| match f {
            Feature::Plants(plants) => Some(plants.clone()),
            _ => None,
        })
        .collect()
}

/// The same world with every plant patch emptied.
fn generator_without_plants(seed: u64) -> Generator {
    let mut worldgen = (*common::load().1).clone();
    for feature in &mut worldgen.features {
        if let Feature::Plants(plants) = feature {
            plants.blocks.clear();
        }
    }
    Generator::new(Seed(seed), Arc::new(worldgen))
}

fn area() -> impl Iterator<Item = ColumnPos> {
    (-4..4).flat_map(|x| (-4..4).map(move |z| ColumnPos { x, z }))
}

fn changes(with: &Column, without: &Column) -> Vec<(u32, i32, u32)> {
    let mut found = Vec::new();
    for x in 0..16 {
        for z in 0..16 {
            for y in 1..511 {
                if with.get(x, y, z) != without.get(x, y, z) {
                    found.push((x, y, z));
                }
            }
        }
    }
    found
}

#[test]
fn plants_grow_only_in_the_air_above_their_ground() {
    for seed in SEEDS {
        let with = common::generator(seed);
        let without = generator_without_plants(seed);
        let features = plant_features(with.content());
        let mut grown = 0;
        for pos in area() {
            let (a, b) = (with.generate(pos), without.generate(pos));
            for (x, y, z) in changes(&a, &b) {
                let plant = a.get(x, y, z);
                let ground = a.get(x, y - 1, z);
                assert_eq!(
                    b.get(x, y, z),
                    BlockId::AIR,
                    "seed {seed}: plant over a block"
                );
                let allowed = features.iter().any(|f| {
                    f.blocks.iter().any(|(block, _)| *block == plant) && f.on.contains(&ground)
                });
                assert!(allowed, "seed {seed}: {plant:?} on {ground:?} in {pos:?}");
                grown += 1;
            }
        }
        assert!(
            grown > 100,
            "seed {seed}: only {grown} plants near the origin"
        );
    }
}

/// Columns lying entirely in the plains (sampled at their corners and centre).
fn plains_columns(generator: &Generator) -> Vec<ColumnPos> {
    let in_plains = |x: i32, z: i32| generator.sample(x, z).1.id.to_string() == "strata:plains";
    (-48..48)
        .flat_map(|x| (-48..48).map(move |z| ColumnPos { x, z }))
        .filter(|pos| {
            let (x, z) = (pos.x * 16, pos.z * 16);
            [(0, 0), (15, 0), (0, 15), (15, 15), (8, 8)]
                .iter()
                .all(|(dx, dz)| in_plains(x + dx, z + dz))
        })
        .take(64)
        .collect()
}

#[test]
fn plains_have_grass_and_flowers() {
    let (content, worldgen) = common::load();
    let generator = Generator::new(Seed(42), worldgen);
    let block = |id: &str| content.blocks.key(&id.parse().unwrap()).unwrap();
    let tall_grass = block("strata:tall_grass");
    let flowers = ["poppy", "dandelion", "cornflower", "daisy", "lavender"]
        .map(|name| block(&format!("strata:{name}")));
    let columns = plains_columns(&generator);
    assert!(columns.len() >= 16, "only {} plains columns", columns.len());
    let mut kinds = std::collections::BTreeSet::new();
    for pos in columns {
        let column = generator.generate(pos);
        for (x, z) in (0..16u32).flat_map(|x| (0..16u32).map(move |z| (x, z))) {
            kinds.extend((60..300).map(|y| column.get(x, y, z)));
        }
    }
    assert!(kinds.contains(&tall_grass), "no tall grass in the plains");
    let found = flowers.iter().filter(|f| kinds.contains(f)).count();
    assert!(found >= 4, "only {found} kinds of flowers in the plains");
}
