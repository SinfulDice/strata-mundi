use super::*;
use crate::settings::Vegetation;
use strata_core::content::{Block, BlockId, RenderKind, TextureId, TextureLayout};
use strata_core::light::PackedLight;
use strata_core::texture::{Connection, Layout};

/// Layers: 0 missing, 1 glass (edges), 2 bookshelf side (vertical), 3 planks, 4 stone
/// (3 random variants). Extra images start at layer 5.
fn layouts() -> Vec<TextureLayout> {
    let layout = |layout, extra, images| TextureLayout {
        layout,
        extra,
        images,
    };
    vec![
        TextureLayout::still(),
        layout(Layout::Connected(Connection::Edges), 0, 16),
        layout(Layout::Connected(Connection::Vertical), 15, 4),
        TextureLayout::still(),
        layout(
            Layout::Random {
                weights: vec![1, 1, 1],
            },
            18,
            3,
        ),
    ]
}

const FIRST_EXTRA: u16 = 5;
const GLASS: u16 = 1;
const BOOKSHELF: u16 = 2;
const PLANKS: u16 = 3;

/// Ids: 0 air, 1 glass, 2 bookshelf, 3 stone.
fn table() -> BlockTable {
    let block = |render, faces: [u16; 6]| Block {
        render,
        faces: faces.map(TextureId),
        ..Block::air()
    };
    let blocks = [
        Block::air(),
        block(RenderKind::Cutout, [1; 6]),
        block(RenderKind::Opaque, [3, 3, 2, 2, 2, 2]),
        block(RenderKind::Opaque, [4; 6]),
    ];
    BlockTable::new(blocks.iter()).with_variants(TextureVariants::from_layouts(layouts()))
}

fn options(texture_variants: bool) -> MeshOptions {
    MeshOptions {
        ambient_occlusion: false,
        smooth_lighting: false,
        vegetation: Vegetation::Full,
        texture_variants,
    }
}

fn mesh_at(origin: [i32; 3], placed: &[([i32; 3], u16)], variants: bool) -> SectionMesh {
    let blocks = Neighborhood::build(|x, y, z| {
        placed
            .iter()
            .find(|(p, _)| *p == [x, y, z])
            .map_or(BlockId::AIR, |(_, id)| BlockId(*id))
    })
    .of_section(
        origin,
        &[BlockId(0), BlockId(1), BlockId(2), BlockId(3)],
        &table(),
    );
    mesh_section(&blocks, &table(), options(variants))
}

fn quads(mesh: &MeshData) -> Vec<[PackedVertex; 4]> {
    mesh.vertices
        .chunks(4)
        .map(|q| [q[0], q[1], q[2], q[3]])
        .collect()
}

fn layer(quad: &[PackedVertex; 4]) -> u16 {
    (quad[0].texture & 0xffff) as u16
}

fn face(quad: &[PackedVertex; 4]) -> u32 {
    (quad[0].geometry >> 15) & 7
}

fn min_corner(quad: &[PackedVertex; 4]) -> [u32; 3] {
    std::array::from_fn(|i| quad.iter().map(|v| v.position()[i]).min().unwrap_or(0))
}

#[test]
fn lone_glass_keeps_its_whole_frame() {
    let mesh = mesh_at([0; 3], &[([5, 5, 5], 1)], true);
    let faces = quads(&mesh.opaque);
    assert_eq!(faces.len(), 6);
    assert!(faces.iter().all(|q| layer(q) == GLASS));
}

#[test]
fn neighbouring_glass_loses_the_border_between_them() {
    let mesh = mesh_at([0; 3], &[([5, 5, 5], 1), ([6, 5, 5], 1)], true);
    let faces = quads(&mesh.opaque);
    assert_eq!(
        faces.len(),
        10,
        "shared faces hidden, joined faces not merged"
    );
    let top = |x| {
        faces
            .iter()
            .find(|q| face(q) == 0 && min_corner(q)[0] == x)
            .map(layer)
    };
    // Up faces: texture right is +x, so the left block joins on its right (bit 2) and
    // the right block on its left (bit 8).
    assert_eq!(top(5), Some(FIRST_EXTRA + 1));
    assert_eq!(top(6), Some(FIRST_EXTRA + 7));
    let ends: Vec<_> = faces.iter().filter(|q| face(q) >= 4).map(layer).collect();
    assert_eq!(ends, [GLASS, GLASS], "end faces have no glass beside them");
}

#[test]
fn stacked_bookshelves_join_vertically() {
    let stack = [([5, 5, 5], 2), ([5, 6, 5], 2), ([5, 7, 5], 2)];
    let mesh = mesh_at([0; 3], &stack, true);
    let side = |y| {
        quads(&mesh.opaque)
            .into_iter()
            .find(|q| face(q) == 2 && min_corner(q)[1] == y)
            .map(|q| layer(&q))
    };
    let vertical = FIRST_EXTRA + 15;
    assert_eq!(side(5), Some(vertical), "bottom: joined above (image 1)");
    assert_eq!(
        side(6),
        Some(vertical + 2),
        "middle: joined both ways (image 3)"
    );
    assert_eq!(side(7), Some(vertical + 1), "top: joined below (image 2)");
    let ends: Vec<_> = quads(&mesh.opaque)
        .into_iter()
        .filter(|q| face(q) <= 1)
        .map(|q| layer(&q))
        .collect();
    assert_eq!(
        ends,
        [PLANKS, PLANKS],
        "planks on top and bottom never connect"
    );
    let alone = mesh_at([0; 3], &[([5, 5, 5], 2)], true);
    assert!(
        quads(&alone.opaque)
            .iter()
            .all(|q| [BOOKSHELF, PLANKS].contains(&layer(q)))
    );
}

fn stone_floor(origin: [i32; 3], variants: bool) -> Vec<[PackedVertex; 4]> {
    let floor: Vec<_> = (0..16)
        .flat_map(|x| (0..16).map(move |z| ([x, 5, z], 3)))
        .collect();
    let mesh = mesh_at(origin, &floor, variants);
    quads(&mesh.opaque)
        .into_iter()
        .filter(|q| face(q) == 0)
        .collect()
}

#[test]
fn random_variants_are_left_to_the_shader_and_faces_still_merge() {
    let tops = stone_floor([32, 64, -48], true);
    assert_eq!(tops.len(), 1, "one merged face");
    assert_eq!(
        layer(&tops[0]),
        4,
        "the base layer: the shader picks the variant"
    );
}

/// Rolling stone surface, as in the meshing benchmark.
fn rolling(x: i32, y: i32, z: i32) -> BlockId {
    let height = 8 + ((x as f32 * 0.4).sin() * 3.0 + (z as f32 * 0.3).cos() * 3.0) as i32;
    BlockId(if y <= height { 3 } else { 0 })
}

/// Before the shader picked variants, this surface took 2 012 vertices with them and
/// 1 816 without (smooth lighting), 1 732 and 1 164 with flat lighting.
#[test]
fn random_variants_add_no_vertex_to_a_surface_section() {
    let count = |variants, shaded| {
        let blocks = Neighborhood::build(rolling)
            .with_light(|_, y, _| PackedLight::new(if y > 8 { 15 } else { 12 }, 0))
            .of_section([0; 3], &[BlockId(0), BlockId(3)], &table());
        let options = MeshOptions {
            ambient_occlusion: shaded,
            smooth_lighting: shaded,
            ..options(variants)
        };
        mesh_section(&blocks, &table(), options)
            .opaque
            .vertices
            .len()
    };
    assert_eq!(count(true, true), count(false, true));
    assert_eq!(count(true, false), count(false, false));
    assert_eq!((count(true, true), count(true, false)), (1816, 1164));
}

#[test]
fn connected_textures_can_be_turned_off() {
    let glass = mesh_at([0; 3], &[([5, 5, 5], 1), ([6, 5, 5], 1)], false);
    assert_eq!(quads(&glass.opaque).len(), 6, "plain glass merges again");
}
