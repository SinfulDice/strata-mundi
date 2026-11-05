//! Liquid faces: the surface follows the levels around each corner (like Minecraft), a
//! liquid never draws the faces it shares with its own family, and a liquid with more of
//! itself above is full. Liquids only hide their neighbours where they are full.

use super::greedy::{CORNERS, FaceFrame, FaceKey, Vec3i, add};
use super::neighborhood::{BlockTable, MeshBlock, MeshFluid, Neighborhood};
use super::shading::{Shading, shade_face};
use strata_core::content::{BlockId, Face, RenderKind};

/// A full block, in sixteenths.
const FULL: u8 = 16;

/// How a face's corners are lowered (liquids); `single` faces are never merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surface {
    /// Sixteenths each corner is lowered by, in the order of `CORNERS`.
    pub drop: [u8; 4],
    pub animation: u8,
    pub single: bool,
}

impl Surface {
    pub const FLAT: Self = Self {
        drop: [0; 4],
        animation: 0,
        single: false,
    };
}

fn at(blocks: &Neighborhood, p: Vec3i) -> BlockId {
    blocks.get(p[0], p[1], p[2])
}

/// Whether `neighbour`, at `front`, hides the `face` of the block behind it. An opaque
/// liquid (lava) is full at the bottom, and on its sides only with more of itself above.
#[inline]
pub fn hides(
    blocks: &Neighborhood,
    table: &BlockTable,
    (front, neighbour): (Vec3i, BlockId),
    face: Face,
) -> bool {
    let block = table.get(neighbour);
    if block.solid_cube {
        return true;
    }
    match block.fluid {
        Some(fluid) if block.render == RenderKind::Opaque => {
            liquid_hides(blocks, table, front, fluid.family, face)
        }
        _ => false,
    }
}

fn liquid_hides(
    blocks: &Neighborhood,
    table: &BlockTable,
    front: Vec3i,
    family: BlockId,
    face: Face,
) -> bool {
    face == Face::Up || is_full(blocks, table, front, family)
}

/// `pos.y` goes up to 15 here: the cell above is still inside the neighbourhood.
fn is_full(blocks: &Neighborhood, table: &BlockTable, pos: Vec3i, family: BlockId) -> bool {
    table.family(at(blocks, add(pos, [0, 1, 0]))) == Some(family)
}

pub fn fluid_face(
    blocks: &Neighborhood,
    table: &BlockTable,
    (face, frame): (Face, &FaceFrame),
    pos: Vec3i,
    (block, fluid): (&MeshBlock, MeshFluid),
    shading: Shading,
) -> Option<FaceKey> {
    let front = add(pos, frame.normal);
    let neighbour = at(blocks, front);
    if table.family(neighbour) == Some(fluid.family) {
        return None;
    }
    let heights = if is_full(blocks, table, pos, fluid.family) {
        [FULL; 4]
    } else {
        corner_heights(blocks, table, pos, fluid.family)
    };
    let drop = CORNERS.map(|(da, db)| {
        let [x, y, z] = corner_offset(frame, da, db);
        if y == 1 {
            FULL - heights[(x + 2 * z) as usize]
        } else {
            0
        }
    });
    let lowered = drop.iter().any(|&d| d > 0);
    if hides(blocks, table, (front, neighbour), face) && !(face == Face::Up && lowered) {
        return None;
    }
    let sloped = drop.iter().any(|&d| d != drop[0]);
    let surface = Surface {
        drop,
        animation: fluid.animation,
        single: sloped || (lowered && face != Face::Up),
    };
    // No ambient occlusion on liquids.
    let shading = Shading {
        ambient_occlusion: false,
        ..shading
    };
    Some(FaceKey::new(
        block.faces[face.index()],
        false,
        shade_face(blocks, table, frame, front, shading),
        block.render == RenderKind::Translucent,
        surface,
    ))
}

/// Where a corner of the face sits in its block, 0 or 1 on each axis.
fn corner_offset(frame: &FaceFrame, da: i32, db: i32) -> Vec3i {
    std::array::from_fn(|i| {
        i32::from(frame.normal[i] > 0) + frame.a[i] * (da + 1) / 2 + frame.b[i] * (db + 1) / 2
    })
}

/// Surface height at the four top corners, indexed by `x + 2 z` (offsets 0 or 1).
fn corner_heights(
    blocks: &Neighborhood,
    table: &BlockTable,
    pos: Vec3i,
    family: BlockId,
) -> [u8; 4] {
    std::array::from_fn(|i| {
        let (cx, cz) = ((i % 2) as i32, (i / 2) as i32);
        corner_height(blocks, table, pos, family, [cx, cz])
    })
}

/// Average of the surfaces of the (up to four) cells of the family around the corner;
/// full when one of them has more of the liquid above.
fn corner_height(
    blocks: &Neighborhood,
    table: &BlockTable,
    pos: Vec3i,
    family: BlockId,
    [cx, cz]: [i32; 2],
) -> u8 {
    let (mut sum, mut count) = (0u32, 0u32);
    for (dx, dz) in [(cx - 1, cz - 1), (cx, cz - 1), (cx - 1, cz), (cx, cz)] {
        let cell = [pos[0] + dx, pos[1], pos[2] + dz];
        let Some(fluid) = table.get(at(blocks, cell)).fluid else {
            continue;
        };
        if fluid.family != family {
            continue;
        }
        if is_full(blocks, table, cell, family) {
            return FULL;
        }
        sum += u32::from(fluid.height);
        count += 1;
    }
    ((sum + count / 2) / count.max(1)) as u8
}

#[cfg(test)]
mod tests {
    use crate::mesher::{BlockTable, MeshOptions, Neighborhood, PackedVertex, mesh_section};
    use crate::settings::Vegetation;
    use strata_core::content::{Block, BlockId, FluidLevel, FluidState, RenderKind, TextureId};

    const STONE: u16 = 1;
    const WATER: u16 = 2;
    const WATER_1: u16 = 3;
    const LAVA: u16 = 4;

    fn liquid(render: RenderKind, source: u16, level: FluidLevel) -> Block {
        Block {
            render,
            faces: [TextureId(source); 6],
            fluid: Some(FluidState {
                source: BlockId(source),
                level,
            }),
            ..Block::air()
        }
    }

    fn table() -> BlockTable {
        let stone = Block {
            render: RenderKind::Opaque,
            faces: [TextureId(STONE); 6],
            ..Block::air()
        };
        let blocks = [
            Block::air(),
            stone,
            liquid(RenderKind::Translucent, WATER, FluidLevel::Source),
            liquid(RenderKind::Translucent, WATER, FluidLevel::Flowing(1)),
            liquid(RenderKind::Opaque, LAVA, FluidLevel::Source),
        ];
        BlockTable::new(blocks.iter())
    }

    fn mesh(placed: &[([i32; 3], u16)]) -> crate::mesher::SectionMesh {
        let blocks = Neighborhood::build(|x, y, z| {
            placed
                .iter()
                .find(|(p, _)| *p == [x, y, z])
                .map_or(BlockId::AIR, |(_, id)| BlockId(*id))
        });
        let options = MeshOptions {
            ambient_occlusion: true,
            smooth_lighting: true,
            vegetation: Vegetation::Full,
        };
        mesh_section(&blocks, &table(), options)
    }

    fn top_corners(vertices: &[PackedVertex], y: u32) -> Vec<([u32; 3], u8)> {
        let mut corners: Vec<_> = vertices
            .iter()
            .filter(|v| v.position()[1] == y && (v.geometry >> 15) & 7 == 0)
            .map(|v| (v.position(), v.drop()))
            .collect();
        corners.sort();
        corners
    }

    #[test]
    fn corners_average_the_levels_around_them() {
        let mesh = mesh(&[([5, 5, 5], WATER), ([6, 5, 5], WATER_1)]);
        let corners = top_corners(&mesh.translucent.vertices, 6);
        let source_side = 16 - 14;
        let shared = 16 - (14 + 12) / 2;
        let flowing_side = 16 - 12;
        assert_eq!(
            corners,
            vec![
                ([5, 6, 5], source_side),
                ([5, 6, 6], source_side),
                ([6, 6, 5], shared),
                ([6, 6, 5], shared),
                ([6, 6, 6], shared),
                ([6, 6, 6], shared),
                ([7, 6, 5], flowing_side),
                ([7, 6, 6], flowing_side),
            ]
        );
    }

    #[test]
    fn a_source_and_its_flow_share_no_face() {
        let mesh = mesh(&[([5, 5, 5], WATER), ([6, 5, 5], WATER_1)]);
        assert_eq!(
            mesh.translucent.quad_count(),
            6 + 6 - 2 - 1,
            "minus the shared sides; the flat bottoms merge"
        );
        assert!(mesh.opaque.is_empty());
    }

    #[test]
    fn a_liquid_under_itself_is_full() {
        let mesh = mesh(&[([5, 5, 5], WATER), ([5, 6, 5], WATER)]);
        let vertices = &mesh.translucent.vertices;
        assert!(
            vertices
                .iter()
                .filter(|v| v.position()[1] <= 6)
                .all(|v| v.drop() == 0),
            "the lower cell reaches the upper one"
        );
        assert_eq!(top_corners(vertices, 6), vec![], "no face in between");
        assert!(top_corners(vertices, 7).iter().all(|&(_, drop)| drop == 2));
        assert_eq!(mesh.translucent.quad_count(), 10);
    }

    #[test]
    fn flat_surfaces_still_merge() {
        let lake: Vec<_> = (0..4)
            .flat_map(|x| (0..4).map(move |z| ([x, 5, z], WATER)))
            .collect();
        let mesh = mesh(&lake);
        let top = top_corners(&mesh.translucent.vertices, 6);
        assert_eq!(top.len(), 4, "one merged quad on a flat lake");
    }

    #[test]
    fn lava_hides_its_neighbours_only_where_it_is_full() {
        let beside = mesh(&[([5, 5, 5], STONE), ([6, 5, 5], LAVA)]);
        assert_eq!(
            beside.opaque.quad_count(),
            6 + 5,
            "stone keeps its side above the low lava surface"
        );
        let deep = mesh(&[([5, 5, 5], STONE), ([6, 5, 5], LAVA), ([6, 6, 5], LAVA)]);
        let stone_faces = deep
            .opaque
            .vertices
            .chunks(4)
            .filter(|q| q.iter().all(|v| v.texture & 0xffff == u32::from(STONE)))
            .count();
        assert_eq!(stone_faces, 5);
    }
}
