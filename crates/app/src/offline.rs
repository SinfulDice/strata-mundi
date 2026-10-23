//! Generating a piece of world without a server, for screenshots.

use std::sync::Arc;
use std::thread;
use strata_client::world::ClientWorld;
use strata_core::coords::{ColumnPos, SECTION_SIZE};
use strata_worldgen::Generator;

/// Every column within `radius` columns of `(x, z)`, generated on all cores.
pub fn generate_around(generator: &Arc<Generator>, x: f64, z: f64, radius: u32) -> ClientWorld {
    let size = f64::from(SECTION_SIZE);
    let center = ((x / size).floor() as i32, (z / size).floor() as i32);
    let r = radius as i32;
    let positions: Vec<ColumnPos> = (-r..=r)
        .flat_map(|dz| (-r..=r).map(move |dx| (dx, dz)))
        .filter(|(dx, dz)| dx * dx + dz * dz <= r * r + r)
        .map(|(dx, dz)| ColumnPos {
            x: center.0 + dx,
            z: center.1 + dz,
        })
        .collect();
    let threads = thread::available_parallelism().map_or(2, |n| n.get());
    let chunk = positions.len().div_ceil(threads).max(1);
    let mut world = ClientWorld::default();
    thread::scope(|scope| {
        let handles: Vec<_> = positions
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    part.iter()
                        .map(|&p| generator.generate(p))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for handle in handles {
            match handle.join() {
                Ok(columns) => columns.into_iter().for_each(|column| world.insert(column)),
                Err(_) => tracing::error!("a generation thread panicked; the capture has a hole"),
            }
        }
    });
    world
}
