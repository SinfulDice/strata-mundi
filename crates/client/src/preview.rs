//! Diorama from `assets/scenes/preview.ron`, shown until the generated world exists
//! (milestones 4-5) and in texture sheets.

use crate::world::ClientWorld;
use serde::Deserialize;
use std::path::Path;
use strata_core::content::Content;
use strata_core::id::Identifier;
use strata_core::pack::{PackError, read_ron};
use strata_protocol::{EntityState, NetEntity};

#[derive(Debug, Deserialize)]
struct SceneFile {
    fills: Vec<Fill>,
    /// Entities shown as they are (a boat on a pond).
    #[serde(default)]
    entities: Vec<SceneEntity>,
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
    for fill in file.fills {
        let block = content
            .blocks
            .key(&fill.block)
            .ok_or_else(|| PackError::invalid(path, format!("unknown block `{}`", fill.block)))?;
        for x in fill.from.0..=fill.to.0 {
            for y in fill.from.1..=fill.to.1 {
                for z in fill.from.2..=fill.to.2 {
                    world.set_block(x, y, z, block);
                }
            }
        }
    }
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
        });
    }
    Ok(world)
}
