use super::vertex::PackedVertex;
use super::{BlockTable, MeshOptions, Neighborhood, SectionMesh, mesh_section};
use crate::settings::Vegetation;
use strata_core::content::{Block, BlockId, RenderKind, TextureId};
use strata_core::light::PackedLight;

/// Ids: 0 air, 1 stone, 2 grass (cross), 3 torch (cross giving off light).
fn table() -> BlockTable {
    let block = |render, layer| Block {
        render,
        faces: [TextureId(layer); 6],
        ..Block::air()
    };
    let blocks = [
        Block::air(),
        block(RenderKind::Opaque, 1),
        block(RenderKind::Cross, 2),
        Block {
            light_emission: 14,
            ..block(RenderKind::Cross, 3)
        },
    ];
    BlockTable::new(blocks.iter())
}

const SMOOTH: MeshOptions = MeshOptions {
    ambient_occlusion: true,
    smooth_lighting: true,
    vegetation: Vegetation::Full,
    texture_variants: true,
    biome_blend: 0,
};

const HARD: MeshOptions = MeshOptions {
    smooth_lighting: false,
    ..SMOOTH
};

/// The four cells above the corner (5, 6, 5) of a stone's top face; dark elsewhere.
const AROUND_CORNER: [([i32; 3], PackedLight); 4] = [
    ([5, 6, 5], PackedLight::new(12, 8)),
    ([4, 6, 5], PackedLight::new(8, 0)),
    ([5, 6, 4], PackedLight::new(4, 4)),
    ([4, 6, 4], PackedLight::new(0, 12)),
];

fn mesh(
    placed: &[([i32; 3], u16)],
    lights: &[([i32; 3], PackedLight)],
    options: MeshOptions,
) -> SectionMesh {
    let find = |list: &[([i32; 3], u16)], p| list.iter().find(|(q, _)| *q == p).map(|e| e.1);
    let blocks =
        Neighborhood::build(|x, y, z| find(placed, [x, y, z]).map_or(BlockId::AIR, BlockId))
            .with_light(|x, y, z| {
                lights
                    .iter()
                    .find(|(p, _)| *p == [x, y, z])
                    .map_or(PackedLight::DARK, |(_, light)| *light)
            });
    mesh_section(&blocks, &table(), options)
}

fn top_vertices(mesh: &SectionMesh, y: u32) -> Vec<PackedVertex> {
    mesh.opaque
        .vertices
        .chunks(4)
        .filter(|quad| quad.iter().all(|v| v.position()[1] == y))
        .flatten()
        .copied()
        .collect()
}

fn light_at_corner(mesh: &SectionMesh, corner: [u32; 3]) -> Option<PackedLight> {
    top_vertices(mesh, corner[1])
        .into_iter()
        .find(|v| v.position() == corner)
        .map(PackedVertex::light)
}

#[test]
fn a_smooth_corner_averages_the_four_cells_around_it() {
    let mesh = mesh(&[([5, 5, 5], 1)], &AROUND_CORNER, SMOOTH);
    assert_eq!(
        light_at_corner(&mesh, [5, 6, 5]),
        Some(PackedLight::new(6, 6)),
        "(12 + 8 + 4 + 0) / 4 and (8 + 0 + 4 + 12) / 4"
    );
}

#[test]
fn opaque_cells_are_left_out_of_the_average() {
    let one_side = mesh(&[([5, 5, 5], 1), ([4, 6, 5], 1)], &AROUND_CORNER, SMOOTH);
    assert_eq!(
        light_at_corner(&one_side, [5, 6, 5]),
        Some(PackedLight::new(5, 8)),
        "(12 + 4 + 0) / 3 and (8 + 4 + 12) / 3"
    );
    let both_sides = mesh(
        &[([5, 5, 5], 1), ([4, 6, 5], 1), ([5, 6, 4], 1)],
        &AROUND_CORNER,
        SMOOTH,
    );
    assert_eq!(
        light_at_corner(&both_sides, [5, 6, 5]),
        Some(PackedLight::new(12, 8)),
        "no light slips diagonally between two opaque sides"
    );
}

#[test]
fn without_smoothing_a_face_takes_the_light_in_front_of_it() {
    let mesh = mesh(&[([5, 5, 5], 1)], &AROUND_CORNER, HARD);
    let top = top_vertices(&mesh, 6);
    assert_eq!(top.len(), 4);
    assert!(top.iter().all(|v| v.light() == PackedLight::new(12, 8)));
}

#[test]
fn faces_with_different_light_are_not_merged() {
    let pair = [([5, 5, 5], 1), ([6, 5, 5], 1)];
    let even = [
        ([5, 6, 5], PackedLight::OPEN_SKY),
        ([6, 6, 5], PackedLight::OPEN_SKY),
    ];
    assert_eq!(top_vertices(&mesh(&pair, &even, HARD), 6).len(), 4);
    let uneven = [
        ([5, 6, 5], PackedLight::OPEN_SKY),
        ([6, 6, 5], PackedLight::new(10, 0)),
    ];
    assert_eq!(top_vertices(&mesh(&pair, &uneven, HARD), 6).len(), 8);
}

#[test]
fn plants_take_the_light_of_their_cell() {
    let mesh = mesh(
        &[([5, 5, 5], 2)],
        &[([5, 5, 5], PackedLight::new(9, 3))],
        SMOOTH,
    );
    assert_eq!(mesh.opaque.vertices.len(), 16);
    assert!(
        mesh.opaque
            .vertices
            .iter()
            .all(|v| v.light() == PackedLight::new(9, 3))
    );
}

#[test]
fn torches_are_drawn_whatever_the_vegetation() {
    let off = MeshOptions {
        vegetation: Vegetation::Off,
        texture_variants: true,
        biome_blend: 0,
        ..SMOOTH
    };
    let placed = [([5, 5, 5], 2), ([7, 5, 5], 3)];
    let mesh = mesh(&placed, &[], off);
    assert_eq!(mesh.opaque.quad_count(), 4, "the torch only");
}
