//! Plants: two diagonal planes through the block, each drawn on both sides so back-face
//! culling keeps them visible from anywhere. They hide nothing and nothing hides them.
//! Wall torches use the same planes, lifted and sheared against their wall.

use super::neighborhood::{BlockTable, Neighborhood, SIZE};
use super::shading::FULL_AO;
use super::vertex::{CROSS_FACE, MeshData, PackedVertex};
use crate::settings::Vegetation;
use strata_core::content::{Face, RenderKind, Tint, TintKind};
use strata_core::light::PackedLight;

pub fn mesh_crosses(
    blocks: &Neighborhood,
    table: &BlockTable,
    vegetation: Vegetation,
    target: &mut MeshData,
) {
    if !blocks.contents().cross {
        return;
    }
    let origin = blocks.origin();
    for y in 0..SIZE as u32 {
        for z in 0..SIZE as u32 {
            for x in 0..SIZE as u32 {
                let block = table.get(blocks.get(x as i32, y as i32, z as i32));
                let world = [x, y, z].map(|c| c as i32);
                let world = std::array::from_fn(|i| origin[i] + world[i]);
                if block.render == RenderKind::Cross && (block.lamp || is_drawn(vegetation, world))
                {
                    let light = blocks.light(x as i32, y as i32, z as i32);
                    let tint = block.face_tint(Face::North).map(|tint| tint.kind());
                    let plant = Plant {
                        cell: [x, y, z],
                        layer: block.faces[Face::North.index()],
                        light,
                        lean: block.lean,
                    };
                    push_cross(target, blocks, plant, tint);
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

struct Plant {
    cell: [u32; 3],
    layer: u16,
    light: PackedLight,
    lean: Option<Face>,
}

/// Sixteenths a wall torch is lifted by (through the liquid `drop`: one block up, then
/// lowered), and how far its foot and top move toward the wall: the top stays further
/// out, so the torch leans away from the wall.
const LEAN_LIFT: u8 = 3;
const LEAN_FOOT: u8 = 7;
const LEAN_TOP: u8 = 2;

fn leaning(vertex: PackedVertex, wall: Face, up: u32) -> PackedVertex {
    let nudge = if up == 0 { LEAN_FOOT } else { LEAN_TOP };
    vertex.with_surface(16 - LEAN_LIFT, 0).leaning(wall, nudge)
}

/// Plants have no ambient occlusion and take the light of their own cell.
fn push_cross(target: &mut MeshData, blocks: &Neighborhood, plant: Plant, tint: Option<TintKind>) {
    let [x, y, z] = plant.cell;
    for (from, to) in [([x, z], [x + 1, z + 1]), ([x + 1, z], [x, z + 1])] {
        let corner = |[cx, cz]: [u32; 2], up: u32, u: u32| {
            let color = tint.map_or(Tint::NEUTRAL, |kind| {
                blocks.tint(kind, (x as i32, z as i32), (cx as i32, cz as i32))
            });
            let lift = u32::from(plant.lean.is_some());
            let position = [cx, y + up + lift, cz];
            let vertex = PackedVertex::new(position, CROSS_FACE, FULL_AO, [u, 1 - up], plant.layer)
                .with_light(plant.light)
                .with_tint(color, false);
            match plant.lean {
                Some(wall) => leaning(vertex, wall, up),
                None => vertex,
            }
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
