use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::Path;
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::coords::ColumnPos;
use strata_core::pack::PackSet;
use strata_worldgen::{Generator, Seed, WorldgenContent};

#[allow(clippy::expect_used)]
fn generator() -> Generator {
    let mut errors = Vec::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut errors);
    let (content, _) = Content::load(&packs, &PackSet::default());
    let terra = "strata:terra".parse().expect("valid identifier");
    let worldgen = WorldgenContent::load(&packs, &content.blocks, &terra, &mut errors)
        .expect("the strata pack defines strata:terra");
    Generator::new(Seed(42), Arc::new(worldgen))
}

fn generation(c: &mut Criterion) {
    let generator = generator();
    let mut x = 0;
    c.bench_function("generate_column", |b| {
        b.iter(|| {
            x += 1;
            generator.generate(black_box(ColumnPos { x, z: 3 }))
        })
    });
}

/// First mountain with overhangs found from the climate, the costliest kind of terrain.
fn mountain(generator: &Generator) -> ColumnPos {
    (0..4000)
        .map(|i| ((i % 60) * 48 - 1440, (i / 60) * 48 - 1600))
        .find(|&(x, z)| generator.sample(x, z).0.overhang > 12.0)
        .map_or(ColumnPos { x: 0, z: 0 }, |(x, z)| ColumnPos {
            x: x.div_euclid(16),
            z: z.div_euclid(16),
        })
}

fn special_columns(c: &mut Criterion) {
    let generator = generator();
    let mountain = mountain(&generator);
    c.bench_function("generate_mountain_column", |b| {
        b.iter(|| generator.generate(black_box(mountain)))
    });
    // A large cavern with an aquifer lake (screenshots/caves-42.png).
    let cavern = ColumnPos { x: -24, z: -24 };
    c.bench_function("generate_cavern_column", |b| {
        b.iter(|| generator.generate(black_box(cavern)))
    });
}

criterion_group!(benches, generation, special_columns);
criterion_main!(benches);
