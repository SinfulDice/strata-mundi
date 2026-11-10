#![allow(clippy::unwrap_used)]

use super::*;
use crate::save::BlockNames;
use std::path::Path;
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::pack::PackSet;
use strata_worldgen::{Generator, Seed, WorldgenContent};

pub fn loader() -> ColumnLoader {
    let mut errors = Vec::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut errors);
    let (content, _) = Content::load(&packs, &PackSet::default());
    let terra = "strata:terra".parse().unwrap();
    let worldgen = WorldgenContent::load(&packs, &content.blocks, &terra, &mut errors).unwrap();
    let generator = Arc::new(Generator::new(Seed(1), Arc::new(worldgen)));
    let names = Arc::new(BlockNames::new(&content.blocks));
    let store = RegionStore::in_memory(names);
    ColumnLoader::new(GenerationWorkers::spawn(2, generator).unwrap(), store)
}

/// Ticks until nothing new arrives for a while; returns the columns sent.
fn settle(loader: &mut ColumnLoader, viewer: &Viewer) -> HashSet<ColumnPos> {
    let mut sent = HashSet::new();
    let mut quiet = 0;
    while quiet < 50 {
        let events = loader.update(viewer);
        quiet = if events.is_empty() { quiet + 1 } else { 0 };
        for event in events {
            match event {
                LoadEvent::Loaded(column, _) => sent.insert(column.pos),
                LoadEvent::Unloaded(pos) => sent.remove(&pos),
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    sent
}

#[test]
fn lowering_the_view_distance_unloads_far_columns() {
    let mut loader = loader();
    let mut viewer = Viewer {
        x: 0.0,
        z: 0.0,
        yaw: 0.0,
        view_distance: 6,
    };
    let sent = settle(&mut loader, &viewer);
    assert!(sent.contains(&ColumnPos { x: 6, z: 0 }));
    viewer.view_distance = 2;
    let events = loader.update(&viewer);
    let unloaded: HashSet<ColumnPos> = events
        .into_iter()
        .filter_map(|e| match e {
            LoadEvent::Unloaded(pos) => Some(pos),
            LoadEvent::Loaded(..) => None,
        })
        .collect();
    assert!(unloaded.contains(&ColumnPos { x: 6, z: 0 }));
    assert!(unloaded.contains(&ColumnPos { x: 5, z: 0 }));
    assert!(
        !unloaded.contains(&ColumnPos { x: 4, z: 0 }),
        "kept by the margin"
    );
}

#[test]
fn changes_survive_unloading_and_reloading() {
    let mut loader = loader();
    let mut viewer = Viewer {
        x: 0.0,
        z: 0.0,
        yaw: 0.0,
        view_distance: 2,
    };
    settle(&mut loader, &viewer);
    let pos = BlockPos::new(3, 300, 3);
    assert!(loader.set_block(pos, BlockId(1)));
    viewer.x = 2000.0;
    settle(&mut loader, &viewer);
    assert_eq!(loader.block(pos), None, "unloaded");
    viewer.x = 0.0;
    settle(&mut loader, &viewer);
    assert_eq!(loader.block(pos), Some(BlockId(1)));
}

#[test]
fn saved_columns_come_back_with_their_generated_biomes() {
    let mut loader = loader();
    let pos = ColumnPos { x: 0, z: 0 };
    let generated = loader.workers.generator().biome_map(pos);
    let mut column = Column::empty(pos);
    column.set(1, 1, 1, BlockId(1));
    loader.store.store(&column);
    let viewer = Viewer {
        x: 8.0,
        z: 8.0,
        yaw: 0.0,
        view_distance: 2,
    };
    loop {
        for event in loader.update(&viewer) {
            if let LoadEvent::Loaded(column, biomes) = event
                && column.pos == pos
            {
                assert_eq!(column.get(1, 1, 1), BlockId(1), "read from the save");
                assert_eq!(biomes, generated);
                return;
            }
        }
        let came = loader.wait_generated(std::time::Duration::from_secs(30));
        assert!(came, "the saved column's biomes never came");
    }
}

#[test]
fn nearest_columns_come_first_and_front_beats_back() {
    let viewer = Viewer {
        x: 8.0,
        z: 8.0,
        yaw: 0.0,
        view_distance: 4,
    };
    let order = wanted(ColumnPos { x: 0, z: 0 }, &viewer);
    assert_eq!(order[0], ColumnPos { x: 0, z: 0 });
    let front = order.iter().position(|p| *p == ColumnPos { x: 0, z: -3 });
    let back = order.iter().position(|p| *p == ColumnPos { x: 0, z: 3 });
    assert!(front < back, "yaw 0 looks north (-Z)");
    assert!(order.iter().all(|p| p.x.abs() <= 4 && p.z.abs() <= 4));
}
