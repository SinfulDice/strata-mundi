//! Plants: two diagonal planes through the block, each drawn on both sides so back-face
//! culling keeps them visible from anywhere. They hide nothing and nothing hides them.

use super::neighborhood::{BlockTable, Neighborhood, SIZE};
use super::vertex::{CROSS_FACE, MeshData, PackedVertex};
use strata_core::content::{Face, RenderKind};

/// Ambient occlusion is not applied to plants yet: every corner fully lit.
const FULL_LIGHT: u8 = 3;

pub fn mesh_crosses(blocks: &Neighborhood, table: &BlockTable, target: &mut MeshData) {
    for y in 0..SIZE as u32 {
        for z in 0..SIZE as u32 {
            for x in 0..SIZE as u32 {
                let block = table.get(blocks.get(x as i32, y as i32, z as i32));
                if block.render == RenderKind::Cross {
                    push_cross(target, [x, y, z], block.faces[Face::North.index()]);
                }
            }
        }
    }
}

fn push_cross(target: &mut MeshData, [x, y, z]: [u32; 3], layer: u16) {
    for (from, to) in [([x, z], [x + 1, z + 1]), ([x + 1, z], [x, z + 1])] {
        let corner = |[cx, cz]: [u32; 2], up: u32, u: u32| {
            PackedVertex::new([cx, y + up, cz], CROSS_FACE, FULL_LIGHT, [u, 1 - up], layer)
        };
        let front = [
            corner(from, 0, 0),
            corner(to, 0, 1),
            corner(to, 1, 1),
            corner(from, 1, 0),
        ];
        let [a, b, c, d] = front;
        target.push_quad(front);
        target.push_quad([b, a, d, c]);
    }
}
