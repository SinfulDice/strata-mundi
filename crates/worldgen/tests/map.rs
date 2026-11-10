#![allow(clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use strata_worldgen::map::{self, MapArea};
use strata_worldgen::{Generator, Seed};

fn generator() -> Generator {
    Generator::new(Seed(42), Arc::clone(&common::load().1))
}

const AREA: MapArea = MapArea {
    center: (100, -40),
    size: (24, 16),
    scale: 8,
};

#[test]
fn parallel_maps_match_the_sequential_one() {
    let generator = generator();
    let sequential = map::render(&generator, AREA);
    assert_eq!(sequential.len(), 24 * 16);
    let parallel = map::render_with(&generator, AREA, 3, &|| false).expect("not cancelled");
    assert_eq!(parallel, sequential);
}

#[test]
fn a_cancelled_map_stops_early() {
    let generator = generator();
    let rows = AtomicUsize::new(0);
    let cancelled = || rows.fetch_add(1, Ordering::Relaxed) >= 4;
    assert!(map::render_with(&generator, AREA, 2, &cancelled).is_none());
    assert!(rows.load(Ordering::Relaxed) < 16);
}
