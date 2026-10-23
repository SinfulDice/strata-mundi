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
