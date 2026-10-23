use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use strata_client::mesher::{BlockTable, Neighborhood, mesh_section};
use strata_core::content::{Block, BlockId, RenderKind, TextureId};

fn table() -> BlockTable {
    let solid = |layer| Block {
        render: RenderKind::Opaque,
        faces: [TextureId(layer); 6],
        ..Block::air()
    };
    let blocks = [Block::air(), solid(1), solid(2), solid(3)];
    BlockTable::new(blocks.iter())
}

/// Rolling terrain surface with three materials: a typical surface section.
fn terrain(x: i32, y: i32, z: i32) -> BlockId {
    let height = 8 + ((x as f32 * 0.4).sin() * 3.0 + (z as f32 * 0.3).cos() * 3.0) as i32;
    match y {
        y if y > height => BlockId::AIR,
        y if y == height => BlockId(1),
        y if y > height - 3 => BlockId(2),
        _ => BlockId(3),
    }
}

/// Worst case: every other block filled, nothing can be merged.
fn checkerboard(x: i32, y: i32, z: i32) -> BlockId {
    BlockId(u16::from((x + y + z).rem_euclid(2) == 0))
}

fn meshing(c: &mut Criterion) {
    let table = table();
    let surface = Neighborhood::build(terrain);
    let worst = Neighborhood::build(checkerboard);
    c.bench_function("mesh_section_terrain", |b| {
        b.iter(|| mesh_section(black_box(&surface), &table, true))
    });
    c.bench_function("mesh_section_checkerboard", |b| {
        b.iter(|| mesh_section(black_box(&worst), &table, true))
    });
    c.bench_function("neighborhood_build", |b| {
        b.iter(|| Neighborhood::build(terrain))
    });
}

criterion_group!(benches, meshing);
criterion_main!(benches);
