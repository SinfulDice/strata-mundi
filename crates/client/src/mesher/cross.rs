//! Plants: two diagonal planes through the block, each drawn on both sides so back-face
//! culling keeps them visible from anywhere. They hide nothing and nothing hides them.

use super::neighborhood::{BlockTable, Neighborhood, SIZE};
use super::vertex::{CROSS_FACE, MeshData, PackedVertex};
use crate::settings::Vegetation;
use strata_core::content::{Face, RenderKind};

/// Ambient occlusion is not applied to plants yet: every corner fully lit.
const FULL_LIGHT: u8 = 3;

pub fn mesh_crosses(
    blocks: &Neighborhood,
    table: &BlockTable,
    vegetation: Vegetation,
    target: &mut MeshData,
) {
    let is_cross = |id| table.get(id).render == RenderKind::Cross;
    if vegetation == Vegetation::Off || !blocks.may_contain(is_cross) {
        return;
    }
    let origin = blocks.origin();
    for y in 0..SIZE as u32 {
        for z in 0..SIZE as u32 {
            for x in 0..SIZE as u32 {
                let block = table.get(blocks.get(x as i32, y as i32, z as i32));
                let world = [x, y, z].map(|c| c as i32);
                let world = std::array::from_fn(|i| origin[i] + world[i]);
                if block.render == RenderKind::Cross && is_drawn(vegetation, world) {
                    push_cross(target, [x, y, z], block.faces[Face::North.index()]);
                }
            }
        }
    }
}

/// Reduced vegetation keeps one plant in `REDUCED_SHARE`, the same ones every time.
fn is_drawn(vegetation: Vegetation, [x, y, z]: [i32; 3]) -> bool {
    match vegetation {
        Vegetation::Full => true,
        Vegetation::Off => false,
        Vegetation::Reduced => position_hash(x, y, z).is_multiple_of(Vegetation::REDUCED_SHARE),
    }
}

fn position_hash(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ (z as u32).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
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
