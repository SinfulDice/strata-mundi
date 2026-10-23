use super::greedy::texture_turned;
use strata_core::content::{Block, BlockId, Face, RenderKind};

pub const SIZE: usize = 16;
const PADDED: usize = SIZE + 2;

/// Copy of a section plus a one-block border taken from its neighbours, so meshing a
/// section never needs to look anything up elsewhere (and can run on any thread).
#[derive(Debug, Clone)]
pub struct Neighborhood {
    blocks: Vec<BlockId>,
}

impl Neighborhood {
    /// `block(x, y, z)` with coordinates from -1 to 16 relative to the section.
    pub fn build(block: impl Fn(i32, i32, i32) -> BlockId) -> Self {
        let mut blocks = Vec::with_capacity(PADDED * PADDED * PADDED);
        for y in -1..=SIZE as i32 {
            for z in -1..=SIZE as i32 {
                for x in -1..=SIZE as i32 {
                    blocks.push(block(x, y, z));
                }
            }
        }
        Self { blocks }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        let p = |c: i32| (c + 1) as usize;
        self.blocks[(p(y) * PADDED + p(z)) * PADDED + p(x)]
    }
}

/// What the mesher needs to know about each block, indexed by `BlockId`.
#[derive(Debug, Clone)]
pub struct BlockTable {
    entries: Vec<MeshBlock>,
}

#[derive(Debug, Clone, Copy)]
pub struct MeshBlock {
    pub render: RenderKind,
    pub cull_same: bool,
    pub faces: [u16; 6],
    /// Faces whose texture is turned a quarter, for blocks lying on their side (logs).
    pub turned: [bool; 6],
}

impl BlockTable {
    pub fn new<'a>(blocks: impl Iterator<Item = &'a Block>) -> Self {
        let entries = blocks
            .map(|block| MeshBlock {
                render: block.render,
                cull_same: block.cull_same,
                faces: Face::ALL.map(|face| block.texture(face).0),
                turned: Face::ALL.map(|face| {
                    block
                        .orientation
                        .is_some_and(|o| texture_turned(face, o.axis))
                }),
            })
            .collect();
        Self { entries }
    }

    pub fn get(&self, id: BlockId) -> MeshBlock {
        self.entries
            .get(usize::from(id.0))
            .copied()
            .unwrap_or(MeshBlock {
                render: RenderKind::Invisible,
                cull_same: true,
                faces: [0; 6],
                turned: [false; 6],
            })
    }

    pub fn is_opaque(&self, id: BlockId) -> bool {
        self.get(id).render == RenderKind::Opaque
    }
}
