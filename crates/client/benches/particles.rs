//! Cost of one frame of particles: moving 2 000 of them and writing their instances.

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use glam::DVec3;
use std::path::Path;
use strata_client::particles::{ParticleSystem, ParticleTable};
use strata_client::render::ParticleInstance;
use strata_client::settings::ParticleMode;
use strata_client::world::ClientWorld;
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::id::Identifier;
use strata_core::pack::PackSet;

const PARTICLES: usize = 2_000;

fn content() -> Content {
    let packs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let mut errors = Vec::new();
    let packs = PackSet::discover(&packs, &mut errors);
    Content::load(&packs, &PackSet::default()).0
}

fn block(content: &Content, name: &str) -> BlockId {
    name.parse::<Identifier>()
        .ok()
        .and_then(|id| content.blocks.key(&id))
        .unwrap_or(BlockId::AIR)
}

/// A stone floor under a ceiling holding water: fragments land, drips fall.
fn cave(content: &Content) -> ClientWorld {
    let (stone, water) = (
        block(content, "strata:stone"),
        block(content, "strata:water"),
    );
    let mut world = ClientWorld::default();
    for x in -16..16 {
        for z in -16..16 {
            world.set_block(x, 0, z, stone);
            world.set_block(x, 9, z, stone);
            world.set_block(x, 10, z, water);
        }
    }
    world
}

fn filled(table: &ParticleTable, world: &ClientWorld, stone: BlockId) -> ParticleSystem {
    let mut system = ParticleSystem::new(table.clone(), ParticleMode::Full);
    let mut i = 0;
    while system.len() < PARTICLES {
        let pos = BlockPos::new(i % 16 - 8, 1 + i / 256 % 6, i / 16 % 16 - 8);
        system.block_changed(pos, stone, BlockId::AIR, world);
        i += 1;
    }
    system
}

fn particle_frame(c: &mut Criterion) {
    let content = content();
    let table = ParticleTable::from_content(&content);
    let world = cave(&content);
    let stone = block(&content, "strata:stone");
    let camera = DVec3::new(0.5, 4.0, 0.5);
    let mut instances: Vec<ParticleInstance> = Vec::with_capacity(4096);
    c.bench_function("particles/frame_2000", |b| {
        b.iter_batched(
            || filled(&table, &world, stone),
            |mut system| {
                system.update(1.0 / 60.0, camera, &world);
                system.write_instances(camera, &world, &mut instances);
                system
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, particle_frame);
criterion_main!(benches);
