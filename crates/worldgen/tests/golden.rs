#![allow(clippy::expect_used)]

//! The default world of a fixed seed, frozen: a change here means existing worlds would
//! get different new chunks. Update `EXPECTED` only for a deliberate change of the world.

mod common;

use strata_core::coords::{ColumnPos, WORLD_HEIGHT};
use strata_core::generation::WorldgenSettings;
use strata_worldgen::{Generator, Seed};

/// Changed by v0.2.1 milestone 1 (rare, narrow ravines with bare rock walls).
const EXPECTED: u64 = 0x8f0a_d9ba_d4cf_19c0;

fn fnv(hash: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
}

#[test]
fn the_default_world_does_not_change() {
    let (content, worldgen) = common::load();
    let generator = Generator::with_settings(Seed(42), &worldgen, &WorldgenSettings::default())
        .expect("default settings");
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for (x, z) in [(0, 0), (-7, 3), (25, -18)] {
        let column = generator.generate(ColumnPos { x, z });
        for y in 0..WORLD_HEIGHT {
            for z in 0..16 {
                for x in 0..16 {
                    let block = content.blocks.identifier(column.get(x, y, z));
                    fnv(
                        &mut hash,
                        block.map_or("?".into(), ToString::to_string).as_bytes(),
                    );
                }
            }
        }
    }
    let spawn = generator.spawn_point();
    for coordinate in [spawn.x, spawn.y, spawn.z] {
        fnv(&mut hash, &coordinate.to_le_bytes());
    }
    assert_eq!(hash, EXPECTED, "the default world changed");
}
