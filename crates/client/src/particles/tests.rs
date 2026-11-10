use super::*;
use strata_core::content::{BlockId, BlockParticles, Particle, ParticleId, TextureId};

const AIR: BlockId = BlockId(0);
const STONE: BlockId = BlockId(1);
const WATER: BlockId = BlockId(2);
const FRAGMENTS: ParticleId = ParticleId(0);
const DRIP: ParticleId = ParticleId(1);
const SPLASH: ParticleId = ParticleId(2);
const BURST: u16 = 30;

fn particle(gravity: f32) -> Particle {
    Particle {
        texture: None,
        color: [255; 4],
        lifetime: [60.0, 60.0],
        size: [0.1, 0.1],
        gravity,
        speed: 0.0,
        drag: 1.0,
        hang: 0.0,
        collides: true,
        glows: false,
        in_liquid: false,
        burst: BURST,
        chance: 1.0,
        impact: None,
    }
}

/// Ids: 0 air, 1 stone (fragments), 2 water (drips that splash).
fn table() -> ParticleTable {
    let stone = ParticleBlock {
        solid: true,
        particles: BlockParticles {
            breaking: Some(FRAGMENTS),
            ..BlockParticles::default()
        },
        faces: [TextureId(3); 6],
        ..ParticleBlock::default()
    };
    let water = ParticleBlock {
        liquid: true,
        particles: BlockParticles {
            drip: Some(DRIP),
            ..BlockParticles::default()
        },
        ..ParticleBlock::default()
    };
    let drip = Particle {
        impact: Some((SPLASH, 4)),
        lifetime: [4.0, 4.0],
        ..particle(14.0)
    };
    let splash = Particle {
        lifetime: [0.3, 0.3],
        ..particle(14.0)
    };
    ParticleTable::new(
        vec![ParticleBlock::default(), stone, water],
        vec![particle(16.0), drip, splash],
    )
}

fn floor() -> ClientWorld {
    let mut world = ClientWorld::default();
    for x in -4..4 {
        for z in -4..4 {
            world.set_block(x, 0, z, STONE);
        }
    }
    world
}

fn system(mode: ParticleMode) -> ParticleSystem {
    ParticleSystem::new(table(), mode)
}

fn run(system: &mut ParticleSystem, world: &ClientWorld, seconds: f32) {
    for _ in 0..(seconds * 60.0) as u32 {
        system.update(1.0 / 60.0, DVec3::new(0.5, 3.0, 0.5), world);
    }
}

fn break_block(system: &mut ParticleSystem, world: &ClientWorld, pos: BlockPos) {
    system.block_changed(pos, STONE, AIR, world);
}

#[test]
fn a_break_throws_as_many_particles_as_the_setting_allows() {
    let world = floor();
    let pos = BlockPos::new(0, 3, 0);
    let mut counts = Vec::new();
    for mode in [ParticleMode::Full, ParticleMode::Reduced, ParticleMode::Off] {
        let mut particles = system(mode);
        break_block(&mut particles, &world, pos);
        counts.push(particles.len());
    }
    assert_eq!(counts, [30, 8, 0]);
}

#[test]
fn only_broken_blocks_throw_fragments() {
    let world = floor();
    let mut particles = system(ParticleMode::Full);
    let pos = BlockPos::new(0, 3, 0);
    particles.block_changed(pos, AIR, STONE, &world);
    particles.block_changed(pos, WATER, AIR, &world);
    particles.block_changed(pos, STONE, STONE, &world);
    assert!(particles.is_empty(), "placing, draining and no change");
    particles.block_changed(pos, STONE, WATER, &world);
    assert_eq!(
        particles.len(),
        usize::from(BURST),
        "washed away by a liquid"
    );
}

#[test]
fn the_pool_never_grows_past_its_capacity() {
    let world = floor();
    let mut particles = system(ParticleMode::Reduced);
    let capacity = particles.capacity();
    assert_eq!(capacity, 1024);
    let mut instances = Vec::with_capacity(capacity);
    for i in 0..200 {
        break_block(&mut particles, &world, BlockPos::new(i % 8 - 4, 3, 0));
        run(&mut particles, &world, 0.05);
        particles.write_instances(DVec3::ZERO, &world, &mut instances);
        assert!(particles.len() <= capacity);
    }
    assert_eq!(particles.len(), capacity, "full after 200 breaks");
    assert_eq!(particles.capacity(), capacity, "never reallocated");
    assert_eq!(
        instances.capacity(),
        capacity,
        "instances reuse their memory"
    );
}

#[test]
fn particles_fall_and_rest_on_full_blocks() {
    let world = floor();
    let mut particles = system(ParticleMode::Full);
    break_block(&mut particles, &world, BlockPos::new(1, 6, 1));
    run(&mut particles, &world, 0.2);
    let falling = particles
        .pool
        .iter()
        .map(|p| p.position.y)
        .fold(9.0, f64::min);
    assert!(falling < 6.0, "they fall: {falling}");
    run(&mut particles, &world, 3.0);
    assert_eq!(particles.len(), usize::from(BURST));
    for particle in particles.pool.iter() {
        assert_eq!(particle.position.y, 1.0, "on top of the floor");
        assert!(particle.on_ground);
    }
}

#[test]
fn particles_without_collision_go_through_blocks() {
    let world = floor();
    let mut table = table();
    table = ParticleTable::new(
        (0..3).map(|i| table.block(BlockId(i))).collect(),
        vec![Particle {
            collides: false,
            ..particle(16.0)
        }],
    );
    let mut particles = ParticleSystem::new(table, ParticleMode::Full);
    break_block(&mut particles, &world, BlockPos::new(1, 3, 1));
    run(&mut particles, &world, 1.5);
    assert!(particles.pool.iter().all(|p| p.position.y < 0.0));
}

/// Water at y 10 over a stone ceiling at y 9, open air below down to the floor.
fn dripping_cave() -> ClientWorld {
    let mut world = floor();
    for x in -4..4 {
        for z in -4..4 {
            world.set_block(x, 9, z, STONE);
            world.set_block(x, 10, z, WATER);
        }
    }
    world
}

#[test]
fn drips_fall_from_ceilings_under_water_and_splash() {
    let world = dripping_cave();
    let mut particles = system(ParticleMode::Full);
    run(&mut particles, &world, 0.5);
    let drips: Vec<_> = particles.pool.iter().filter(|p| p.kind == DRIP).collect();
    assert!(!drips.is_empty(), "drips appeared");
    for drip in &drips {
        assert!(drip.position.y > 1.0 && drip.position.y < 9.0, "{drip:?}");
    }
    run(&mut particles, &world, 1.5);
    assert!(
        particles.pool.iter().any(|p| p.kind == SPLASH),
        "drips splash on the floor"
    );
    let off = &mut system(ParticleMode::Off);
    run(off, &world, 2.0);
    assert!(off.is_empty(), "no particle at all when off");
}

#[test]
fn a_dry_ceiling_never_drips() {
    let mut world = floor();
    for x in -4..4 {
        for z in -4..4 {
            world.set_block(x, 9, z, STONE);
        }
    }
    let mut particles = system(ParticleMode::Full);
    run(&mut particles, &world, 2.0);
    assert!(particles.is_empty());
}

#[test]
fn switching_the_setting_resizes_the_pool_once() {
    let world = floor();
    let mut particles = system(ParticleMode::Full);
    break_block(&mut particles, &world, BlockPos::new(0, 3, 0));
    particles.set_mode(ParticleMode::Full);
    assert_eq!(particles.len(), usize::from(BURST), "same size: kept");
    particles.set_mode(ParticleMode::Off);
    assert_eq!((particles.len(), particles.capacity()), (0, 0));
    break_block(&mut particles, &world, BlockPos::new(0, 3, 0));
    assert!(particles.is_empty());
}
