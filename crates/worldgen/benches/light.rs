use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::Path;
use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::{BlockId, Content};
use strata_core::coords::{BlockPos, ColumnMap, ColumnPos};
use strata_core::light::{LightTable, Lighting};
use strata_core::pack::PackSet;
use strata_worldgen::{Generator, Seed, WorldgenContent};

#[allow(clippy::expect_used)]
fn setup() -> (Generator, Content) {
    let mut errors = Vec::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut errors);
    let (content, _) = Content::load(&packs, &PackSet::default());
    let terra = "strata:terra".parse().expect("valid identifier");
    let worldgen = WorldgenContent::load(&packs, &content.blocks, &terra, &mut errors)
        .expect("the strata pack defines strata:terra");
    (Generator::new(Seed(42), Arc::new(worldgen)), content)
}

const CENTER: ColumnPos = ColumnPos { x: 3, z: 3 };

/// The 3×3 generated columns around `CENTER`, the outer ones already lit.
fn lit_neighbours(generator: &Generator, table: &LightTable) -> (ColumnMap<Column>, Lighting) {
    let mut world = ColumnMap::default();
    for dz in -1..=1 {
        for dx in -1..=1 {
            let pos = ColumnPos {
                x: CENTER.x + dx,
                z: CENTER.z + dz,
            };
            world.insert(pos, generator.generate(pos));
        }
    }
    let mut lighting = Lighting::default();
    for &pos in world.keys() {
        if pos != CENTER {
            lighting.light_column(&world, table, pos);
        }
    }
    (world, lighting)
}

fn light(c: &mut Criterion) {
    let (generator, content) = setup();
    let table = LightTable::from_content(&content);
    let (mut world, mut lighting) = lit_neighbours(&generator, &table);
    c.bench_function("light_generated_column", |b| {
        b.iter(|| {
            lighting.remove_column(CENTER);
            lighting.light_column(&world, &table, black_box(CENTER))
        })
    });
    lighting.light_column(&world, &table, CENTER);
    let torch = "strata:torch"
        .parse()
        .ok()
        .and_then(|id| content.blocks.key(&id))
        .unwrap_or(BlockId::AIR);
    let surface = (0..512)
        .rev()
        .find(|&y| world[&CENTER].get(15, y, 8) != BlockId::AIR)
        .unwrap_or(100);
    let pos = BlockPos::new(CENTER.x * 16 + 15, surface + 1, CENTER.z * 16 + 8);
    let set = |world: &mut ColumnMap<Column>, block| {
        if let Some(column) = world.get_mut(&CENTER) {
            column.set(15, pos.y, 8, block);
        }
    };
    c.bench_function("light_place_and_break_torch", |b| {
        b.iter(|| {
            set(&mut world, torch);
            lighting.block_changed(&world, &table, pos, BlockId::AIR);
            set(&mut world, BlockId::AIR);
            lighting.block_changed(&world, &table, pos, torch);
        })
    });
}

criterion_group!(benches, light);
criterion_main!(benches);
