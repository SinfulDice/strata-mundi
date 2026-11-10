//! Turns sections of blocks into GPU-ready meshes.

mod cross;
mod face_key;
mod fluid;
mod greedy;
mod neighborhood;
mod shading;
#[cfg(test)]
mod shading_tests;
mod tints;
mod variants;
#[cfg(test)]
mod variants_tests;
mod vertex;
mod workers;

pub use greedy::{MeshOptions, SectionMesh, mesh_section};
pub use neighborhood::{BlockTable, LightSections, MeshBlock, Neighborhood};
pub use tints::{ColumnTints, MAX_BLEND_RADIUS};
pub use variants::TextureVariants;
pub use vertex::{MeshData, PackedVertex};
pub use workers::{MeshJob, MeshResult, MeshWorkers, SectionKey};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Vegetation;
    use strata_core::content::{Block, BlockId, Face, RenderKind, TextureId};

    fn block(render: RenderKind, layer: u16) -> Block {
        Block {
            render,
            faces: [TextureId(layer); 6],
            ..Block::air()
        }
    }

    /// Ids: 0 air, 1 stone, 2 glass (cutout), 3 water (translucent), 4 dirt, 5 grass (cross).
    fn table() -> BlockTable {
        let blocks = [
            Block::air(),
            block(RenderKind::Opaque, 1),
            block(RenderKind::Cutout, 2),
            block(RenderKind::Translucent, 3),
            block(RenderKind::Opaque, 4),
            block(RenderKind::Cross, 5),
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
        mesh_section(&blocks, &table(), AO)
    }

    const AO: MeshOptions = MeshOptions {
        ambient_occlusion: true,
        smooth_lighting: true,
        vegetation: Vegetation::Full,
        texture_variants: true,
        biome_blend: 0,
    };

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

    fn winding(quad: &[PackedVertex; 4]) -> [i32; 3] {
        let p = quad.map(|v| v.position().map(|c| c as i32));
        let e1 = [0, 1, 2].map(|i| p[1][i] - p[0][i]);
        let e2 = [0, 1, 2].map(|i| p[3][i] - p[0][i]);
        [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ]
    }

    #[test]
    fn a_cross_is_two_diagonals_seen_from_both_sides() {
        let mesh = mesh(&[([5, 5, 5], 5)]);
        assert!(mesh.translucent.is_empty());
        let quads: Vec<_> = quads(&mesh.opaque).collect();
        assert_eq!(quads.len(), 4);
        for pair in quads.chunks(2) {
            let (front, back) = (winding(&pair[0]), winding(&pair[1]));
            assert_eq!(
                front,
                back.map(|c| -c),
                "the same plane, facing the other way"
            );
            assert_eq!(front[1], 0, "upright plane");
            assert!(front[0] != 0 && front[2] != 0, "diagonal plane");
            for vertex in pair[0] {
                let [x, y, z] = vertex.position();
                assert!((5..=6).contains(&x) && (5..=6).contains(&y) && (5..=6).contains(&z));
                assert_eq!((vertex.geometry >> 15) & 7, vertex::CROSS_FACE as u32);
                assert_eq!(vertex.texture & 0xffff, 5);
            }
        }
        assert_ne!(
            winding(&quads[0]),
            winding(&quads[2]),
            "two different diagonals"
        );
    }

    #[test]
    fn crosses_hide_nothing_and_are_never_hidden() {
        let beside = mesh(&[([5, 5, 5], 1), ([6, 5, 5], 5)]);
        assert_eq!(
            beside.opaque.quad_count(),
            6 + 4,
            "stone keeps all its faces"
        );
        let walled: Vec<_> = Face::ALL
            .iter()
            .map(|face| {
                let (dx, dy, dz) = face.offset();
                ([5 + dx, 5 + dy, 5 + dz], 1)
            })
            .chain([([5, 5, 5], 5)])
            .collect();
        let crosses = quads(&mesh(&walled).opaque)
            .filter(|q| (q[0].geometry >> 15) & 7 == vertex::CROSS_FACE as u32)
            .count();
        assert_eq!(crosses, 4, "a plant shut in by stone is still meshed");
    }

    /// A meadow: a plant on every cell of layer 5, meshed with `vegetation`.
    fn meadow(vegetation: Vegetation, origin: [i32; 3]) -> usize {
        let blocks = Neighborhood::build(|_, y, _| BlockId(if y == 5 { 5 } else { 0 })).of_section(
            origin,
            &[BlockId(0), BlockId(5)],
            &table(),
        );
        let options = MeshOptions { vegetation, ..AO };
        mesh_section(&blocks, &table(), options).opaque.quad_count() / 4
    }

    #[test]
    fn vegetation_can_be_reduced_or_turned_off() {
        let full = meadow(Vegetation::Full, [0; 3]);
        assert_eq!(full, 256);
        let reduced = meadow(Vegetation::Reduced, [0; 3]);
        assert!((50..120).contains(&reduced), "about a third: {reduced}");
        assert_eq!(
            reduced,
            meadow(Vegetation::Reduced, [0; 3]),
            "always the same plants"
        );
        assert_ne!(reduced, 0);
        assert_eq!(meadow(Vegetation::Off, [0; 3]), 0);
    }

    #[test]
    fn sections_without_plants_skip_the_cross_pass() {
        let blocks = Neighborhood::build(|_, y, _| BlockId(if y == 5 { 5 } else { 0 })).of_section(
            [0; 3],
            &[BlockId(0), BlockId(1)],
            &table(),
        );
        assert!(mesh_section(&blocks, &table(), AO).is_empty());
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
