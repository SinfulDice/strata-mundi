//! Turns sections of blocks into GPU-ready meshes.

mod greedy;
mod neighborhood;
mod vertex;
mod workers;

pub use greedy::{SectionMesh, mesh_section};
pub use neighborhood::{BlockTable, MeshBlock, Neighborhood};
pub use vertex::{MeshData, PackedVertex};
pub use workers::{MeshJob, MeshResult, MeshWorkers, SectionKey};

#[cfg(test)]
mod tests {
    use super::*;
    use strata_core::content::{Block, BlockId, RenderKind, TextureId};

    fn block(render: RenderKind, layer: u16) -> Block {
        Block {
            render,
            faces: [TextureId(layer); 6],
            ..Block::air()
        }
    }

    /// Ids: 0 air, 1 stone, 2 glass (cutout), 3 water (translucent), 4 dirt.
    fn table() -> BlockTable {
        let blocks = [
            Block::air(),
            block(RenderKind::Opaque, 1),
            block(RenderKind::Cutout, 2),
            block(RenderKind::Translucent, 3),
            block(RenderKind::Opaque, 4),
        ];
        BlockTable::new(blocks.iter())
    }

    fn mesh(placed: &[([i32; 3], u16)]) -> SectionMesh {
        let blocks = Neighborhood::build(|x, y, z| {
            placed
                .iter()
                .find(|(p, _)| *p == [x, y, z])
                .map_or(BlockId::AIR, |(_, id)| BlockId(*id))
        });
        mesh_section(&blocks, &table(), true)
    }

    #[test]
    fn single_cube_has_six_faces() {
        let mesh = mesh(&[([5, 5, 5], 1)]);
        assert_eq!(mesh.opaque.quad_count(), 6);
        assert!(mesh.translucent.is_empty());
    }

    #[test]
    fn touching_cubes_hide_shared_faces_and_merge() {
        let different = mesh(&[([5, 5, 5], 1), ([6, 5, 5], 4)]);
        assert_eq!(
            different.opaque.quad_count(),
            10,
            "2 × 6 faces minus the 2 shared ones"
        );
        let same = mesh(&[([5, 5, 5], 1), ([6, 5, 5], 1)]);
        assert_eq!(
            same.opaque.quad_count(),
            6,
            "greedy merge of the 4 long sides"
        );
    }

    #[test]
    fn lying_logs_turn_the_bark_of_their_long_sides_only() {
        use super::greedy::texture_turned;
        use strata_core::content::{Axis, Face};
        let turned = |axis| Face::ALL.map(|face| texture_turned(face, axis));
        assert_eq!(turned(Axis::Y), [false; 6]);
        // Up, Down, North, South, East, West
        assert_eq!(turned(Axis::X), [true, true, true, true, false, false]);
        assert_eq!(turned(Axis::Z), [false, false, false, false, true, true]);
    }

    #[test]
    fn transparent_blocks_do_not_hide_neighbours() {
        let mesh = mesh(&[([5, 5, 5], 1), ([6, 5, 5], 2)]);
        assert_eq!(
            mesh.opaque.quad_count(),
            6 + 5,
            "stone keeps its side facing the glass"
        );
    }

    #[test]
    fn identical_transparent_blocks_hide_their_shared_face() {
        let water = mesh(&[([5, 5, 5], 3), ([6, 5, 5], 3)]);
        assert_eq!(water.translucent.quad_count(), 6);
        assert!(water.opaque.is_empty());
    }

    #[test]
    fn faces_on_section_borders_check_the_neighbour() {
        let hidden = mesh(&[([15, 5, 5], 1), ([16, 5, 5], 1)]);
        assert_eq!(
            hidden.opaque.quad_count(),
            5,
            "the block across the border is not meshed here"
        );
    }

    #[test]
    fn occluded_corners_are_darker() {
        // The block above-right of (5,5,5) darkens the right edge of its top face only.
        let mesh = mesh(&[([5, 5, 5], 1), ([6, 6, 5], 1)]);
        let top = quads(&mesh.opaque)
            .find(|q| {
                q.iter()
                    .all(|v| v.position()[1] == 6 && v.position()[0] <= 6)
            })
            .expect("top face of the lower block");
        for vertex in top {
            let expected = if vertex.position()[0] == 6 { 2 } else { 3 };
            assert_eq!(vertex.ao(), expected, "{:?}", vertex.position());
        }
    }

    fn quads(mesh: &MeshData) -> impl Iterator<Item = [PackedVertex; 4]> + '_ {
        mesh.vertices.chunks(4).map(|q| [q[0], q[1], q[2], q[3]])
    }

    fn face_normal(vertex: PackedVertex) -> [i32; 3] {
        [
            [0, 1, 0],
            [0, -1, 0],
            [0, 0, -1],
            [0, 0, 1],
            [1, 0, 0],
            [-1, 0, 0],
        ][((vertex.geometry >> 15) & 7) as usize]
    }

    #[test]
    fn triangles_face_outward() {
        let mesh = mesh(&[([5, 5, 5], 1), ([6, 5, 5], 1)]);
        for triangle in mesh.opaque.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| mesh.opaque.vertices[triangle[i] as usize]);
            let p = [a, b, c].map(|v| v.position().map(|c| c as i32));
            let e1 = [0, 1, 2].map(|i| p[1][i] - p[0][i]);
            let e2 = [0, 1, 2].map(|i| p[2][i] - p[0][i]);
            let cross = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let normal = face_normal(a);
            assert!(cross.iter().zip(normal).map(|(c, n)| c * n).sum::<i32>() > 0);
        }
    }

    #[test]
    fn merged_faces_repeat_the_texture() {
        let mesh = mesh(&[([5, 5, 5], 1), ([6, 5, 5], 1)]);
        let top = quads(&mesh.opaque)
            .find(|q| face_normal(q[0]) == [0, 1, 0])
            .expect("top face");
        let uvs = top.map(|v| ((v.geometry >> 20) & 31, (v.geometry >> 25) & 31));
        assert_eq!(uvs.iter().map(|uv| uv.0).max(), Some(2), "2 blocks wide");
        assert_eq!(uvs.iter().map(|uv| uv.1).max(), Some(1), "1 block deep");
        assert_eq!(uvs.iter().map(|uv| uv.0.min(uv.1)).min(), Some(0));
    }
}
