//! Per-corner shading of a face: ambient occlusion and smooth light, both read from the
//! cells around the corner on the face's side (like Minecraft).

use super::greedy::{CORNERS, FaceFrame, Vec3i, add};
use super::neighborhood::{BlockTable, Neighborhood};
use strata_core::light::PackedLight;

/// The graphics settings that change face shading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shading {
    pub ambient_occlusion: bool,
    /// Off: every corner takes the light of the cell in front of the face.
    pub smooth_lighting: bool,
}

/// Ambient occlusion (0 darkest to 3) and light of each corner, in the order of `CORNERS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceShading {
    pub ao: [u8; 4],
    pub light: [PackedLight; 4],
}

pub const FULL_AO: u8 = 3;

/// Shading of the face whose front cell (the one it looks into) is `front`.
pub fn shade_face(
    blocks: &Neighborhood,
    table: &BlockTable,
    frame: &FaceFrame,
    front: Vec3i,
    shading: Shading,
) -> FaceShading {
    let front_light = light_at(blocks, front);
    let mut result = FaceShading {
        ao: [FULL_AO; 4],
        light: [front_light; 4],
    };
    if !shading.ambient_occlusion && !shading.smooth_lighting {
        return result;
    }
    let ring = Ring::around(blocks, table, frame, front, shading.smooth_lighting);
    // Every clear cell as bright as the front one (open air, solid rock): nothing to blend.
    let smooth = shading.smooth_lighting && !ring.lit_like(front_light);
    for (i, (da, db)) in CORNERS.into_iter().enumerate() {
        let (side_a, side_b, corner) = (ring.cell(da, 0), ring.cell(0, db), ring.cell(da, db));
        let opaque = [side_a, side_b, corner].map(|c| c.0);
        if shading.ambient_occlusion {
            result.ao[i] = vertex_ao(opaque[0], opaque[1], opaque[2]);
        }
        if smooth {
            // Light cannot slip diagonally between two opaque sides.
            let hidden = (opaque[0] && opaque[1]).then_some((true, corner.1));
            let cells = [side_a, side_b, hidden.unwrap_or(corner)];
            result.light[i] = smooth_light(front_light, cells);
        }
    }
    result
}

/// Opacity and light of the 3×3 cells around the front cell, in the face's plane: each
/// is shared by up to four corners, so it is read once.
struct Ring {
    cells: [(bool, PackedLight); 9],
}

impl Ring {
    fn around(
        blocks: &Neighborhood,
        table: &BlockTable,
        frame: &FaceFrame,
        front: Vec3i,
        with_light: bool,
    ) -> Self {
        let cells = std::array::from_fn(|i| {
            let (da, db) = (i as i32 % 3 - 1, i as i32 / 3 - 1);
            let p = add(add(front, frame.a.map(|c| c * da)), frame.b.map(|c| c * db));
            let opaque = table.is_opaque(at(blocks, p));
            let light = if with_light && !opaque {
                light_at(blocks, p)
            } else {
                PackedLight::DARK
            };
            (opaque, light)
        });
        Self { cells }
    }

    fn lit_like(&self, light: PackedLight) -> bool {
        self.cells
            .iter()
            .all(|&(opaque, cell)| opaque || cell == light)
    }

    #[inline]
    fn cell(&self, da: i32, db: i32) -> (bool, PackedLight) {
        self.cells[((db + 1) * 3 + da + 1) as usize]
    }
}

/// Average of the front cell and the clear cells among the three others, per channel.
fn smooth_light(front: PackedLight, cells: [(bool, PackedLight); 3]) -> PackedLight {
    let (mut sky, mut block, mut count) = (u32::from(front.sky()), u32::from(front.block()), 1);
    for (opaque, light) in cells {
        if !opaque {
            sky += u32::from(light.sky());
            block += u32::from(light.block());
            count += 1;
        }
    }
    let average = |sum: u32| ((sum + count / 2) / count) as u8;
    PackedLight::new(average(sky), average(block))
}

/// 3 = fully lit, 0 = darkest corner (both sides blocked).
fn vertex_ao(side_a: bool, side_b: bool, corner: bool) -> u8 {
    if side_a && side_b {
        0
    } else {
        FULL_AO - u8::from(side_a) - u8::from(side_b) - u8::from(corner)
    }
}

#[inline]
fn at(blocks: &Neighborhood, p: Vec3i) -> strata_core::content::BlockId {
    blocks.get(p[0], p[1], p[2])
}

#[inline]
pub fn light_at(blocks: &Neighborhood, p: Vec3i) -> PackedLight {
    blocks.light(p[0], p[1], p[2])
}
