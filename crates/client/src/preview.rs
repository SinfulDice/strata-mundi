//! Dioramas from `assets/scenes/`: `preview.ron`, shown until the generated world exists
//! and in texture sheets, and the scenes of `--scene` captures (edits and liquid ticks
//! simulated before the capture).

use crate::world::ClientWorld;
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::id::Identifier;
use strata_core::pack::{PackError, read_ron};
use strata_core::updates::{BlockAccess, BlockUpdates, FluidSettings, UpdateRules};
use strata_protocol::{EntityState, NetEntity};

#[derive(Debug, Deserialize)]
struct SceneFile {
    fills: Vec<Fill>,
    /// Entities shown as they are (a boat on a pond).
    #[serde(default)]
    entities: Vec<SceneEntity>,
    /// Changes made after the fills, which wake the liquids around them (a breach).
    #[serde(default)]
    edits: Vec<Fill>,
    /// Liquid ticks simulated after the edits, so a capture shows the flow.
    #[serde(default)]
    ticks: u32,
}

#[derive(Debug, Deserialize)]
struct SceneEntity {
    kind: Identifier,
    position: (f64, f64, f64),
    #[serde(default)]
    yaw: f32,
}

#[derive(Debug, Deserialize)]
struct Fill {
    from: (i32, i32, i32),
    to: (i32, i32, i32),
    block: Identifier,
}

/// Fills are applied in order; later ones overwrite earlier ones.
pub fn load_preview(path: &Path, content: &Content) -> Result<ClientWorld, PackError> {
    let file: SceneFile = read_ron(path)?;
    let mut world = ClientWorld::default();
    let mut bounds = Bounds::default();
    for fill in &file.fills {
        let block = fill_block(path, content, fill)?;
        bounds.include(fill);
        for (x, y, z) in fill.cells() {
            world.set_block(x, y, z, block);
        }
    }
    let mut scene = SceneAccess {
        world: &mut world,
        bounds,
    };
    simulate(&mut scene, path, content, &file.edits, file.ticks)?;
    for (index, entity) in file.entities.into_iter().enumerate() {
        let kind = content
            .entities
            .key(&entity.kind)
            .ok_or_else(|| PackError::invalid(path, format!("unknown entity `{}`", entity.kind)))?;
        let (x, y, z) = entity.position;
        world.entities.spawn(EntityState {
            id: NetEntity(index as u64),
            kind,
            position: [x, y, z],
            yaw: entity.yaw,
            item: None,
        });
    }
    Ok(world)
}

fn fill_block(path: &Path, content: &Content, fill: &Fill) -> Result<BlockId, PackError> {
    content
        .blocks
        .key(&fill.block)
        .ok_or_else(|| PackError::invalid(path, format!("unknown block `{}`", fill.block)))
}

impl Fill {
    fn cells(&self) -> impl Iterator<Item = (i32, i32, i32)> + use<> {
        let (from, to) = (self.from, self.to);
        (from.0..=to.0).flat_map(move |x| {
            (from.1..=to.1).flat_map(move |y| (from.2..=to.2).map(move |z| (x, y, z)))
        })
    }
}

/// The edits go through the server's block updates, then the liquids flow for `ticks`.
fn simulate(
    scene: &mut SceneAccess,
    path: &Path,
    content: &Content,
    edits: &[Fill],
    ticks: u32,
) -> Result<(), PackError> {
    let rules = Arc::new(UpdateRules::new(content));
    let mut updates = BlockUpdates::new(rules, FluidSettings::default());
    let mut changes = Vec::new();
    for edit in edits {
        let block = fill_block(path, content, edit)?;
        for (x, y, z) in edit.cells() {
            let pos = BlockPos::new(x, y, z);
            if !scene.bounds.contains(pos) {
                return Err(PackError::invalid(
                    path,
                    format!("edit at {x},{y},{z} outside the scene's fills"),
                ));
            }
            updates.set_block(scene, pos, block, &mut changes);
            changes.clear();
        }
    }
    for _ in 0..ticks {
        if updates.active() == 0 {
            break;
        }
        updates.tick(scene, &mut changes);
        changes.clear();
    }
    Ok(())
}

type Corner = (i32, i32, i32);

/// Box holding every fill (none before the first): liquids reaching its edge wait as at
/// unloaded columns.
#[derive(Default)]
struct Bounds(Option<(Corner, Corner)>);

impl Bounds {
    fn include(&mut self, fill: &Fill) {
        let (a, b) = (fill.from, fill.to);
        let low = (a.0.min(b.0), a.1.min(b.1), a.2.min(b.2));
        let high = (a.0.max(b.0), a.1.max(b.1), a.2.max(b.2));
        let (min, max) = self.0.unwrap_or((low, high));
        self.0 = Some((
            (min.0.min(low.0), min.1.min(low.1), min.2.min(low.2)),
            (max.0.max(high.0), max.1.max(high.1), max.2.max(high.2)),
        ));
    }

    fn contains(&self, pos: BlockPos) -> bool {
        self.0.is_some_and(|(min, max)| {
            (min.0..=max.0).contains(&pos.x)
                && (min.1..=max.1).contains(&pos.y)
                && (min.2..=max.2).contains(&pos.z)
        })
    }
}

struct SceneAccess<'a> {
    world: &'a mut ClientWorld,
    bounds: Bounds,
}

impl BlockAccess for SceneAccess<'_> {
    fn block(&self, pos: BlockPos) -> Option<BlockId> {
        self.bounds.contains(pos).then(|| self.world.block_at(pos))
    }

    fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        let inside = self.bounds.contains(pos);
        if inside {
            self.world.set_block(pos.x, pos.y, pos.z, block);
        }
        inside
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use strata_core::pack::PackSet;

    #[test]
    fn the_breach_scene_flows_before_the_capture() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let packs = PackSet::discover(&assets.join("packs"), &mut Vec::new());
        let content = Content::load(&packs, &PackSet::default()).0;
        let world = load_preview(&assets.join("scenes/breach.ron"), &content).unwrap();
        let rules = UpdateRules::new(&content);
        let wet = |x, y, z| rules.fluid_state(world.block_at(BlockPos::new(x, y, z)));
        assert!(wet(-9, 12, 0).is_some(), "the breach lets the lake out");
        assert!(wet(-6, 12, 0).is_some(), "the stream runs down the steps");
        assert!(wet(-6, 12, 3).is_none(), "without widening");
    }

    #[test]
    fn bounds_start_at_the_first_fill() {
        let fill = |from, to| Fill {
            from,
            to,
            block: "strata:air".parse().unwrap(),
        };
        let mut bounds = Bounds::default();
        assert!(!bounds.contains(BlockPos::new(0, 0, 0)));
        bounds.include(&fill((5, 5, 5), (3, 6, 3)));
        assert!(bounds.contains(BlockPos::new(4, 6, 5)));
        assert!(
            !bounds.contains(BlockPos::new(0, 0, 0)),
            "not stretched to the origin"
        );
        bounds.include(&fill((-2, 5, 3), (-1, 5, 3)));
        assert!(bounds.contains(BlockPos::new(0, 5, 4)));
    }
}
