//! Diorama from `assets/scenes/preview.ron`, shown until the generated world exists
//! (milestones 4-5) and in texture sheets.

use crate::world::ClientWorld;
use serde::Deserialize;
use std::path::Path;
use strata_core::content::Content;
use strata_core::id::Identifier;
use strata_core::pack::{PackError, read_ron};

#[derive(Debug, Deserialize)]
struct SceneFile {
    fills: Vec<Fill>,
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
    Ok(world)
}
