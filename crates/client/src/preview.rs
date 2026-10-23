//! Diorama from `assets/scenes/preview.ron`, shown until the chunk world exists
//! (milestones 3-4) and in texture sheets.

use crate::render::MeshData;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use strata_core::content::{BlockId, Content, Face};
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

pub struct PreviewScene {
    blocks: HashMap<(i32, i32, i32), BlockId>,
}

const NEIGHBOURS: [(Face, (i32, i32, i32)); 6] = [
    (Face::Up, (0, 1, 0)),
    (Face::Down, (0, -1, 0)),
    (Face::North, (0, 0, -1)),
    (Face::South, (0, 0, 1)),
    (Face::East, (1, 0, 0)),
    (Face::West, (-1, 0, 0)),
];

impl PreviewScene {
    pub fn load(path: &Path, content: &Content) -> Result<Self, PackError> {
        let file: SceneFile = read_ron(path)?;
        let mut blocks = HashMap::new();
        for fill in file.fills {
            let block = content.blocks.key(&fill.block).ok_or_else(|| {
                PackError::invalid(path, format!("unknown block `{}`", fill.block))
            })?;
            for x in fill.from.0..=fill.to.0 {
                for y in fill.from.1..=fill.to.1 {
                    for z in fill.from.2..=fill.to.2 {
                        blocks.insert((x, y, z), block);
                    }
                }
            }
        }
        Ok(Self { blocks })
    }

    /// Faces hidden by an opaque neighbour are skipped.
    pub fn mesh(&self, content: &Content) -> MeshData {
        let mut mesh = MeshData::default();
        let block = |pos| self.blocks.get(&pos).and_then(|&id| content.blocks.get(id));
        let mut positions: Vec<_> = self.blocks.keys().copied().collect();
        positions.sort_unstable();
        for (x, y, z) in positions {
            let Some(current) = block((x, y, z)).filter(|b| b.render.is_visible()) else {
                continue;
            };
            for (face, (dx, dy, dz)) in NEIGHBOURS {
                let hidden = block((x + dx, y + dy, z + dz)).is_some_and(|n| n.is_opaque());
                if !hidden {
                    mesh.push_face([x as f32, y as f32, z as f32], face, current.texture(face));
                }
            }
        }
        mesh
    }
}
