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

criterion_group!(benches, generation);
criterion_main!(benches);
