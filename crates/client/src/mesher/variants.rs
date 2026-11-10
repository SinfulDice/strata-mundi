//! Textures the mesher picks per face: random variants by block position and connected
//! textures by neighbours. Animations are left to the shader.

use super::greedy::{FaceFrame, Vec3i, add};
use super::neighborhood::Neighborhood;
use strata_core::content::{BlockId, TextureLayout, TextureLibrary, position_hash};
use strata_core::texture::{Connection, Layout};

/// Texture layouts by layer, with the GPU layer of each texture's second image.
#[derive(Debug, Clone, Default)]
pub struct TextureVariants {
    layouts: Vec<TextureLayout>,
    second: Vec<u16>,
}

impl TextureVariants {
    pub fn from_library(library: &TextureLibrary) -> Self {
        Self::from_layouts(library.layouts.clone())
    }

    /// One layout per texture, in layer order: extra images start after the last one
    /// (as in `TextureLibrary`).
    pub fn from_layouts(layouts: Vec<TextureLayout>) -> Self {
        let first_extra = layouts.len();
        let second = layouts
            .iter()
            .map(|layout| u16::try_from(first_extra + layout.extra as usize).unwrap_or(0))
            .collect();
        Self { layouts, second }
    }

    /// Whether faces with this texture change from block to block.
    pub fn varies(&self, layer: u16) -> bool {
        self.layouts.get(usize::from(layer)).is_some_and(|layout| {
            layout.images > 1
                && matches!(layout.layout, Layout::Random { .. } | Layout::Connected(_))
        })
    }

    fn image(&self, layer: u16, index: u32) -> u16 {
        match self.second.get(usize::from(layer)) {
            Some(&second) if index > 0 => second + (index - 1) as u16,
            _ => layer,
        }
    }

    /// Layer of the face of block `id` at `pos` (section coordinates) whose texture is
    /// `layer`. Only faces of the same variant merge, so the greedy pass stays correct.
    pub(super) fn face_layer(
        &self,
        blocks: &Neighborhood,
        (frame, pos): (&FaceFrame, Vec3i),
        id: BlockId,
        layer: u16,
    ) -> u16 {
        let Some(layout) = self.layouts.get(usize::from(layer)) else {
            return layer;
        };
        let index = match &layout.layout {
            Layout::Random { .. } => {
                let [x, y, z] = blocks.origin();
                layout.random_index(position_hash(x + pos[0], y + pos[1], z + pos[2]))
            }
            Layout::Connected(connect) => connection_mask(blocks, frame, pos, id, *connect),
            Layout::Still | Layout::Animated { .. } => 0,
        };
        self.image(layer, index.min(layout.images.saturating_sub(1)))
    }
}

/// Sides joining the same block, as the bits of `Connection`. A neighbour whose own face
/// is covered by the same block in front does not join (the inner edge of an L stays).
fn connection_mask(
    blocks: &Neighborhood,
    frame: &FaceFrame,
    pos: Vec3i,
    id: BlockId,
    connect: Connection,
) -> u32 {
    let joins = |dir: Vec3i| {
        let side = add(pos, dir);
        let front = add(side, frame.normal);
        blocks.get(side[0], side[1], side[2]) == id
            && blocks.get(front[0], front[1], front[2]) != id
    };
    let neg = |v: Vec3i| v.map(|c| -c);
    match connect {
        Connection::Edges => [neg(frame.down), frame.right, frame.down, neg(frame.right)]
            .into_iter()
            .enumerate()
            .map(|(bit, dir)| u32::from(joins(dir)) << bit)
            .sum(),
        Connection::Vertical if frame.normal[1] == 0 => {
            u32::from(joins([0, 1, 0])) | (u32::from(joins([0, -1, 0])) << 1)
        }
        Connection::Vertical => 0,
    }
}
