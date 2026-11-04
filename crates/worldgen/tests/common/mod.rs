#![allow(dead_code)]
#![allow(clippy::expect_used)]

use std::path::Path;
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::pack::PackSet;
use strata_worldgen::{Generator, Seed, WorldgenContent};

pub fn load() -> (Content, Arc<WorldgenContent>) {
    let mut errors = Vec::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut errors);
    let (content, more) = Content::load(&packs, &PackSet::default());
    errors.extend(more);
    let terra = "strata:terra".parse().expect("valid id");
    let worldgen = WorldgenContent::load(&packs, &content.blocks, &terra, &mut errors);
    assert!(errors.is_empty(), "{errors:#?}");
    (content, Arc::new(worldgen.expect("terra exists")))
}

pub fn generator(seed: u64) -> Generator {
    Generator::new(Seed(seed), load().1)
}

/// Same world, without caves: whatever is left open comes from the relief.
pub fn generator_without_caves(seed: u64) -> Generator {
    let mut worldgen = (*load().1).clone();
    worldgen.terrain.caves = false;
    Generator::new(Seed(seed), Arc::new(worldgen))
}

/// Blocks placed by trees and pillars, and liquids, to tell the ground apart from what
/// grows or flows on it.
pub fn feature_blocks(worldgen: &WorldgenContent) -> Vec<strata_core::content::BlockId> {
    let terrain = &worldgen.terrain;
    worldgen
        .features
        .iter()
        .flat_map(|feature| match feature {
            strata_worldgen::Feature::Tree { log, leaves, .. } => vec![*log, *leaves],
            strata_worldgen::Feature::Pillar { block, .. } => vec![*block],
            strata_worldgen::Feature::Ore { .. } => Vec::new(),
        })
        .chain([terrain.water, terrain.ice])
        .collect()
}

/// Columns of a mountainous region, found from the climate.
pub fn rugged_columns(generator: &Generator, count: usize) -> Vec<strata_core::coords::ColumnPos> {
    (0..4000)
        .map(|i| ((i % 60) * 48 - 1440, (i / 60) * 48 - 1600))
        .filter(|&(x, z)| generator.sample(x, z).0.overhang > 12.0)
        .take(count)
        .map(|(x, z)| strata_core::coords::ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        })
        .collect()
}

/// River bed points well inland.
pub fn river_points(generator: &Generator) -> Vec<(i32, i32)> {
    let sea_level = generator.content().terrain.sea_level;
    (0..10000)
        .map(|i| ((i % 100) * 24 - 1200, (i / 100) * 24 - 1200))
        .filter(|&(x, z)| {
            let (sample, biome) = generator.sample(x, z);
            biome.river && sample.climate.continentalness > -0.15 && sample.height < sea_level
        })
        .collect()
}
