use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use strata_client::mesher::{BlockTable, MeshOptions, Neighborhood, TextureVariants, mesh_section};
use strata_core::content::{
    Block, BlockId, FluidLevel, FluidState, RenderKind, TextureId, TextureLayout,
};
use strata_core::light::PackedLight;
use strata_core::texture::Layout;

fn table() -> BlockTable {
    let solid = |layer| Block {
        render: RenderKind::Opaque,
        faces: [TextureId(layer); 6],
        ..Block::air()
    };
    let water = |level| Block {
        render: RenderKind::Translucent,
        faces: [TextureId(4); 6],
        fluid: Some(FluidState {
            source: BlockId(WATER),
            level,
        }),
        ..Block::air()
    };
    let flowing = (1..=7).map(|n| water(FluidLevel::Flowing(n)));
    let blocks: Vec<Block> = [Block::air(), solid(1), solid(2), solid(3)]
        .into_iter()
        .chain([water(FluidLevel::Source)])
        .chain(flowing)
        .collect();
    BlockTable::new(blocks.iter()).with_variants(variants())
}

/// Grass, dirt and stone with three random variants each, like the game.
fn variants() -> TextureVariants {
    let random = |extra| TextureLayout {
        layout: Layout::Random {
            weights: vec![1; 3],
        },
        extra,
        images: 3,
    };
    let still = TextureLayout::still();
    TextureVariants::from_layouts(vec![still.clone(), random(0), random(2), random(4), still])
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

const WATER: u16 = 4;

/// The rolling terrain flooded up to y = 10 by water at every level: sloped surfaces that
/// cannot be merged.
fn flowing_water(x: i32, y: i32, z: i32) -> BlockId {
    match terrain(x, y, z) {
        BlockId::AIR if y <= 10 => BlockId(WATER + (x + 2 * z).rem_euclid(8) as u16),
        block => block,
    }
}

/// Worst case: every other block filled, nothing can be merged.
fn checkerboard(x: i32, y: i32, z: i32) -> BlockId {
    BlockId(u16::from((x + y + z).rem_euclid(2) == 0))
}

/// Light read from the neighbourhood like in game (open sky, dimmer low down).
fn daylight(_: i32, y: i32, _: i32) -> PackedLight {
    PackedLight::new(if y > 8 { 15 } else { 12 }, 0)
}

const AO: MeshOptions = MeshOptions {
    ambient_occlusion: true,
    smooth_lighting: true,
    vegetation: strata_client::settings::Vegetation::Full,
    texture_variants: true,
};

fn meshing(c: &mut Criterion) {
    let table = table();
    // Real sections know their palette, which lets the mesher skip the liquid checks.
    let land: Vec<BlockId> = (0..WATER).map(BlockId).collect();
    let flooded: Vec<BlockId> = (0..WATER + 8).map(BlockId).collect();
    let surface = Neighborhood::build(terrain)
        .with_light(daylight)
        .of_section([0; 3], &land, &table);
    let worst = Neighborhood::build(checkerboard)
        .with_light(daylight)
        .of_section([0; 3], &land, &table);
    let water = Neighborhood::build(flowing_water)
        .with_light(daylight)
        .of_section([0; 3], &flooded, &table);
    c.bench_function("mesh_section_terrain", |b| {
        b.iter(|| mesh_section(black_box(&surface), &table, AO))
    });
    c.bench_function("mesh_section_checkerboard", |b| {
        b.iter(|| mesh_section(black_box(&worst), &table, AO))
    });
    c.bench_function("mesh_section_flowing_water", |b| {
        b.iter(|| mesh_section(black_box(&water), &table, AO))
    });
    c.bench_function("neighborhood_build", |b| {
        b.iter(|| Neighborhood::build(terrain))
    });
}

criterion_group!(benches, meshing);
criterion_main!(benches);
