use super::*;
use crate::settings::Vegetation;
use strata_core::content::{Block, BlockId, RenderKind, TextureId, TextureLayout, position_hash};
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

/// Expected layer of the stone top face at section cell `(x, z)` of a floor at y = 5.
fn stone_layer(origin: [i32; 3], x: u32, z: u32) -> u16 {
    let hash = position_hash(origin[0] + x as i32, origin[1] + 5, origin[2] + z as i32);
    match layouts()[4].random_index(hash) {
        0 => 4,
        index => FIRST_EXTRA + 18 + index as u16 - 1,
    }
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
fn random_variants_are_stable_and_merge_only_alike_faces() {
    let origin = [32, 64, -48];
    let tops = stone_floor(origin, true);
    assert_eq!(
        tops,
        stone_floor(origin, true),
        "same position, same variants"
    );
    assert!(tops.len() > 3, "several variants: {}", tops.len());
    let mut covered = 0;
    let mut seen = std::collections::HashSet::new();
    for quad in &tops {
        let [x0, _, z0] = min_corner(quad);
        let [x1, _, z1] =
            std::array::from_fn(|i| quad.iter().map(|v| v.position()[i]).max().unwrap_or(0));
        for x in x0..x1 {
            for z in z0..z1 {
                assert_eq!(stone_layer(origin, x, z), layer(quad), "cell {x},{z}");
                covered += 1;
            }
        }
        seen.insert(layer(quad));
    }
    assert_eq!(covered, 256, "every cell exactly once");
    assert_eq!(seen.len(), 3, "all three variants appear");
    assert_ne!(
        tops,
        stone_floor([48, 64, -48], true),
        "another place, other variants"
    );
}

#[test]
fn variants_can_be_turned_off() {
    let tops = stone_floor([0; 3], false);
    assert_eq!(tops.len(), 1, "one merged face");
    assert_eq!(layer(&tops[0]), 4);
    let glass = mesh_at([0; 3], &[([5, 5, 5], 1), ([6, 5, 5], 1)], false);
    assert_eq!(quads(&glass.opaque).len(), 6, "plain glass merges again");
}
