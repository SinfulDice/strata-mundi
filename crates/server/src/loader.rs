//! Keeps the columns around the player loaded: nearest and in front first, generated in
//! the background, sent to the client, and forgotten once far behind.

use crate::generation::GenerationWorkers;
use std::collections::{HashMap, HashSet};
use strata_core::chunk::Column;
use strata_core::coords::{ColumnPos, SECTION_SIZE};

/// Columns sent per tick at most, so a burst never floods the client.
const SENT_PER_TICK: usize = 48;
/// Columns are dropped only this far beyond the view distance, so moving back and forth
/// along a border does not reload them.
const UNLOAD_MARGIN: i32 = 2;

/// Where the player is, as last reported.
#[derive(Debug, Clone, Copy)]
pub struct Viewer {
    pub x: f64,
    pub z: f64,
    pub yaw: f32,
    pub view_distance: u32,
}

pub enum LoadEvent {
    Loaded(Column),
    Unloaded(ColumnPos),
}

pub struct ColumnLoader {
    workers: GenerationWorkers,
    /// Load order for the last centre, heading sector and distance (recomputed on change).
    order: Option<((ColumnPos, i32, u32), Vec<ColumnPos>)>,
    columns: HashMap<ColumnPos, Column>,
    requested: HashSet<ColumnPos>,
    sent: HashSet<ColumnPos>,
}

impl ColumnLoader {
    pub fn new(workers: GenerationWorkers) -> Self {
        Self {
            workers,
            order: None,
            columns: HashMap::new(),
            requested: HashSet::new(),
            sent: HashSet::new(),
        }
    }

    pub fn update(&mut self, viewer: &Viewer) -> Vec<LoadEvent> {
        for column in self.workers.finished() {
            self.requested.remove(&column.pos);
            self.columns.insert(column.pos, column);
        }
        let center = column_of(viewer);
        let mut events = self.unload_far(center, viewer.view_distance);
        let wanted = self.load_order(center, viewer);
        self.request_missing(&wanted);
        let ready: Vec<ColumnPos> = wanted
            .iter()
            .filter(|p| !self.sent.contains(p) && self.columns.contains_key(p))
            .take(SENT_PER_TICK)
            .copied()
            .collect();
        for pos in ready {
            if let Some(column) = self.columns.get(&pos) {
                self.sent.insert(pos);
                events.push(LoadEvent::Loaded(column.clone()));
            }
        }
        events
    }

    fn load_order(&mut self, center: ColumnPos, viewer: &Viewer) -> Vec<ColumnPos> {
        let sector = (viewer.yaw / std::f32::consts::FRAC_PI_4).round() as i32;
        let key = (center, sector.rem_euclid(8), viewer.view_distance);
        match &self.order {
            Some((cached, order)) if *cached == key => order.clone(),
            _ => {
                let order = wanted(center, viewer);
                self.order = Some((key, order.clone()));
                order
            }
        }
    }

    fn request_missing(&mut self, wanted: &[ColumnPos]) {
        let room = self.workers.capacity().saturating_sub(self.requested.len());
        let missing: Vec<ColumnPos> = wanted
            .iter()
            .filter(|p| !self.columns.contains_key(p) && !self.requested.contains(p))
            .take(room)
            .copied()
            .collect();
        for pos in missing {
            if !self.workers.request(pos) {
                break;
            }
            self.requested.insert(pos);
        }
    }

    fn unload_far(&mut self, center: ColumnPos, view_distance: u32) -> Vec<LoadEvent> {
        let limit = view_distance as i32 + UNLOAD_MARGIN;
        let far = |p: &ColumnPos| (p.x - center.x).abs().max((p.z - center.z).abs()) > limit;
        let gone: Vec<ColumnPos> = self.columns.keys().copied().filter(far).collect();
        self.requested.retain(|p| !far(p));
        gone.into_iter()
            .filter_map(|pos| {
                self.columns.remove(&pos);
                self.sent.remove(&pos).then_some(LoadEvent::Unloaded(pos))
            })
            .collect()
    }
}

fn column_of(viewer: &Viewer) -> ColumnPos {
    let size = f64::from(SECTION_SIZE);
    ColumnPos {
        x: (viewer.x / size).floor() as i32,
        z: (viewer.z / size).floor() as i32,
    }
}

/// Columns within a circle of `view_distance`, nearest first, those in front of the
/// player before those behind at equal distance.
fn wanted(center: ColumnPos, viewer: &Viewer) -> Vec<ColumnPos> {
    let r = viewer.view_distance as i32;
    let (sin, cos) = viewer.yaw.sin_cos();
    let forward = (sin, -cos);
    let mut columns: Vec<(i64, ColumnPos)> = (-r..=r)
        .flat_map(|dz| (-r..=r).map(move |dx| (dx, dz)))
        .filter(|(dx, dz)| dx * dx + dz * dz <= r * r + r)
        .map(|(dx, dz)| {
            let distance = (dx * dx + dz * dz) as f32;
            let facing = (dx as f32 * forward.0 + dz as f32 * forward.1) / distance.sqrt().max(1.0);
            let priority = (distance * (1.25 - 0.5 * facing) * 16.0) as i64;
            (
                priority,
                ColumnPos {
                    x: center.x + dx,
                    z: center.z + dz,
                },
            )
        })
        .collect();
    columns.sort_by_key(|(priority, pos)| (*priority, pos.x, pos.z));
    columns.into_iter().map(|(_, pos)| pos).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Arc;
    use strata_core::content::Content;
    use strata_core::pack::PackSet;
    use strata_worldgen::{Generator, Seed, WorldgenContent};

    fn loader() -> ColumnLoader {
        let mut errors = Vec::new();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut errors);
        let (content, _) = Content::load(&packs, &PackSet::default());
        let terra = "strata:terra".parse().unwrap();
        let worldgen = WorldgenContent::load(&packs, &content.blocks, &terra, &mut errors).unwrap();
        let generator = Arc::new(Generator::new(Seed(1), Arc::new(worldgen)));
        ColumnLoader::new(GenerationWorkers::spawn(2, generator).unwrap())
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
                    LoadEvent::Loaded(column) => sent.insert(column.pos),
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
                LoadEvent::Loaded(_) => None,
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
}
