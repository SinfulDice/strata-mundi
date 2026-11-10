//! Section mesher: face culling, greedy merging of identical neighbouring faces, and
//! per-vertex ambient occlusion. Pure CPU code, run on worker threads.

use super::cross::mesh_crosses;
use super::face_key::FaceKey;
use super::fluid::{Surface, fluid_face, hides};
use super::neighborhood::{BlockTable, MeshBlock, Neighborhood, SIZE};
use super::shading::{Shading, shade_face};
use super::vertex::{MeshData, PackedVertex};
use crate::settings::{GraphicsSettings, Vegetation};
use strata_core::content::{Axis, Face, RenderKind, Tint};

pub(super) type Vec3i = [i32; 3];

/// Opaque and cutout faces go to `opaque`; blended ones (water, ice) to `translucent`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SectionMesh {
    pub opaque: MeshData,
    pub translucent: MeshData,
}

impl SectionMesh {
    pub fn is_empty(&self) -> bool {
        self.opaque.is_empty() && self.translucent.is_empty()
    }
}

/// How each face sits in space: normal, the two in-plane axes the greedy pass walks
/// (`a`, `b`), and the texture directions (`right`, `down`) as seen from outside.
pub(super) struct FaceFrame {
    pub(super) normal: Vec3i,
    pub(super) a: Vec3i,
    pub(super) b: Vec3i,
    pub(super) right: Vec3i,
    pub(super) down: Vec3i,
}

fn frame(face: Face) -> FaceFrame {
    let (x, y, z) = ([1, 0, 0], [0, 1, 0], [0, 0, 1]);
    let neg = |v: Vec3i| [-v[0], -v[1], -v[2]];
    match face {
        Face::Up => FaceFrame {
            normal: y,
            a: x,
            b: z,
            right: x,
            down: z,
        },
        Face::Down => FaceFrame {
            normal: neg(y),
            a: x,
            b: z,
            right: x,
            down: neg(z),
        },
        Face::North => FaceFrame {
            normal: neg(z),
            a: x,
            b: y,
            right: neg(x),
            down: neg(y),
        },
        Face::South => FaceFrame {
            normal: z,
            a: x,
            b: y,
            right: x,
            down: neg(y),
        },
        Face::East => FaceFrame {
            normal: x,
            a: z,
            b: y,
            right: neg(z),
            down: neg(y),
        },
        Face::West => FaceFrame {
            normal: neg(x),
            a: z,
            b: y,
            right: z,
            down: neg(y),
        },
    }
}

/// Textures run along `down`; a block lying along the face's `right` direction needs its
/// texture turned so the grain follows the block.
pub fn texture_turned(face: Face, axis: Axis) -> bool {
    frame(face).right[axis as usize] != 0
}

/// Corner offsets along the frame's `a` and `b` axes, in quad order.
pub(super) const CORNERS: [(i32, i32); 4] = [(-1, -1), (1, -1), (1, 1), (-1, 1)];

/// The graphics settings that change meshes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MeshOptions {
    /// Off: every corner is fully lit, which also lets more faces merge.
    pub ambient_occlusion: bool,
    /// Off: a face takes the light of the cell in front of it, with hard steps.
    pub smooth_lighting: bool,
    pub vegetation: Vegetation,
    /// Off: connected textures show their first image (fewer, larger merged faces).
    /// Random variants are picked by the shader and merge either way.
    pub texture_variants: bool,
    /// Blocks over which biome colours blend (0: sharp borders); used when gathering the
    /// neighbourhood.
    pub biome_blend: u32,
}

impl MeshOptions {
    pub fn from_graphics(graphics: &GraphicsSettings) -> Self {
        Self {
            ambient_occlusion: graphics.ambient_occlusion,
            smooth_lighting: graphics.smooth_lighting,
            vegetation: graphics.vegetation,
            texture_variants: graphics.texture_variants,
            biome_blend: graphics.biome_blend,
        }
    }

    fn shading(self) -> Shading {
        Shading {
            ambient_occlusion: self.ambient_occlusion,
            smooth_lighting: self.smooth_lighting,
        }
    }
}

pub fn mesh_section(
    blocks: &Neighborhood,
    table: &BlockTable,
    options: MeshOptions,
) -> SectionMesh {
    let mut mesh = SectionMesh::default();
    // Liquid cells need their own checks; sections without any skip them entirely.
    if blocks.contents().liquid {
        mesh_faces::<true>(blocks, table, options, &mut mesh);
    } else {
        mesh_faces::<false>(blocks, table, options, &mut mesh);
    }
    mesh_crosses(blocks, table, options.vegetation, &mut mesh.opaque);
    mesh
}

fn mesh_faces<const LIQUIDS: bool>(
    blocks: &Neighborhood,
    table: &BlockTable,
    options: MeshOptions,
    mesh: &mut SectionMesh,
) {
    for face in Face::ALL {
        let frame = frame(face);
        for depth in 0..SIZE as i32 {
            let mut mask = [[None; SIZE]; SIZE];
            for (b, row) in mask.iter_mut().enumerate() {
                for (a, cell) in row.iter_mut().enumerate() {
                    let pos = cell_position(&frame, depth, a as i32, b as i32);
                    *cell = face_key::<LIQUIDS>(blocks, table, face, &frame, pos, options);
                }
            }
            merge_slice(&mut mask, |rect, key| {
                let target = if key.translucent() {
                    &mut mesh.translucent
                } else {
                    &mut mesh.opaque
                };
                emit_quad(target, blocks, (face, &frame), (depth, rect), key);
            });
        }
    }
}

/// Block position for cell `(a, b)` of slice `depth` along the face normal.
fn cell_position(frame: &FaceFrame, depth: i32, a: i32, b: i32) -> Vec3i {
    let axis = |v: Vec3i| v.map(i32::abs);
    let n = axis(frame.normal);
    std::array::from_fn(|i| n[i] * depth + frame.a[i] * a + frame.b[i] * b)
}

pub(super) fn add(p: Vec3i, q: Vec3i) -> Vec3i {
    [p[0] + q[0], p[1] + q[1], p[2] + q[2]]
}

fn face_key<const LIQUIDS: bool>(
    blocks: &Neighborhood,
    table: &BlockTable,
    face: Face,
    frame: &FaceFrame,
    pos: Vec3i,
    options: MeshOptions,
) -> Option<FaceKey> {
    let shading = options.shading();
    let id = blocks.get(pos[0], pos[1], pos[2]);
    let block = table.get(id);
    if matches!(block.render, RenderKind::Invisible | RenderKind::Cross) {
        return None;
    }
    if LIQUIDS && let Some(fluid) = block.fluid {
        let key = fluid_face(blocks, table, (face, frame), pos, (block, fluid), shading)?;
        return Some(tinted(key, blocks, block, (face, frame), pos));
    }
    let front = add(pos, frame.normal);
    let neighbour = blocks.get(front[0], front[1], front[2]);
    if hides(blocks, table, (front, neighbour), face) || (neighbour == id && block.cull_same) {
        return None;
    }
    let mut layer = block.faces[face.index()];
    if block.varied && options.texture_variants {
        layer = table.variants().face_layer(blocks, (frame, pos), id, layer);
    }
    let key = FaceKey::new(
        layer,
        block.turned[face.index()],
        shade_face(blocks, table, frame, front, shading),
        block.render == RenderKind::Translucent,
        Surface::FLAT,
    );
    Some(tinted(key, blocks, block, (face, frame), pos))
}

fn tinted(
    key: FaceKey,
    blocks: &Neighborhood,
    block: &MeshBlock,
    (face, frame): (Face, &FaceFrame),
    pos: Vec3i,
) -> FaceKey {
    let Some(tint) = block.face_tint(face) else {
        return key;
    };
    let span = |axis: usize| {
        let n = frame.normal[axis];
        if n == 0 {
            [pos[axis], pos[axis] + 1]
        } else {
            let plane = pos[axis] + i32::from(n > 0);
            [plane, plane]
        }
    };
    let (xs, zs) = (span(0), span(2));
    let at = |cx: i32, cz: i32| blocks.tint(tint.kind(), (pos[0], pos[2]), (cx, cz));
    let corners = if blocks.tints_blended() {
        [
            at(xs[0], zs[0]),
            at(xs[1], zs[0]),
            at(xs[1], zs[1]),
            at(xs[0], zs[1]),
        ]
    } else {
        [at(xs[0], zs[0]); 4]
    };
    key.with_tint(tint, corners)
}

/// Rectangle in slice coordinates: `(a, b, width, height)`.
type Rect = (usize, usize, usize, usize);

/// Greedy merge: grow each unvisited face first along `a`, then along `b`, while keys match.
fn merge_slice(mask: &mut [[Option<FaceKey>; SIZE]; SIZE], mut emit: impl FnMut(Rect, FaceKey)) {
    for b in 0..SIZE {
        let mut a = 0;
        while a < SIZE {
            let Some(key) = mask[b][a] else {
                a += 1;
                continue;
            };
            let (width, height) = if key.single() {
                (1, 1)
            } else {
                let width = (a..SIZE).take_while(|&i| mask[b][i] == Some(key)).count();
                let height = (b..SIZE)
                    .take_while(|&j| (a..a + width).all(|i| mask[j][i] == Some(key)))
                    .count();
                (width, height)
            };
            for row in mask.iter_mut().skip(b).take(height) {
                row[a..a + width].fill(None);
            }
            emit((a, b, width, height), key);
            a += width;
        }
    }
}

fn emit_quad(
    target: &mut MeshData,
    blocks: &Neighborhood,
    (face, frame): (Face, &FaceFrame),
    (depth, rect): (i32, Rect),
    key: FaceKey,
) {
    let (a0, b0, w, h) = (rect.0 as i32, rect.1 as i32, rect.2 as i32, rect.3 as i32);
    let positive = frame.normal.iter().sum::<i32>() > 0;
    let plane = depth + i32::from(positive);
    let n = frame.normal.map(i32::abs);
    let corner = |a: i32, b: i32| -> Vec3i {
        std::array::from_fn(|i| n[i] * plane + frame.a[i] * a + frame.b[i] * b)
    };
    let mut corners = [
        (corner(a0, b0), 0),
        (corner(a0 + w, b0), 1),
        (corner(a0 + w, b0 + h), 2),
        (corner(a0, b0 + h), 3),
    ];
    if !faces_outward(corners.map(|c| c.0), frame.normal) {
        corners.reverse();
    }
    let dot = |p: Vec3i, v: Vec3i| p[0] * v[0] + p[1] * v[1] + p[2] * v[2];
    let min_u = corners
        .iter()
        .map(|c| dot(c.0, frame.right))
        .min()
        .unwrap_or(0);
    let min_v = corners
        .iter()
        .map(|c| dot(c.0, frame.down))
        .min()
        .unwrap_or(0);
    let width = if dot(frame.right, frame.a) != 0 { w } else { h };
    let tint = key.tint();
    let vertices = corners.map(|(p, i)| {
        let (u, v) = (dot(p, frame.right) - min_u, dot(p, frame.down) - min_v);
        let uv = if key.turned() { [v, width - u] } else { [u, v] }.map(|c| c as u32);
        PackedVertex::new(
            p.map(|c| c as u32),
            face.index(),
            key.ao(i),
            uv,
            key.layer(),
        )
        .with_surface(key.drop(i), key.animation())
        .with_light(key.light(i))
        .with_tint(
            tint.map_or(Tint::NEUTRAL, |(kind, _, whole)| {
                whole.unwrap_or_else(|| blocks.tint(kind, (p[0], p[2]), (p[0], p[2])))
            }),
            tint.is_some_and(|(_, masked, _)| masked),
        )
    });
    target.push_quad(vertices);
}

fn faces_outward(corners: [Vec3i; 4], normal: Vec3i) -> bool {
    let e1 = [0, 1, 2].map(|i| corners[1][i] - corners[0][i]);
    let e2 = [0, 1, 2].map(|i| corners[3][i] - corners[0][i]);
    let cross = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    cross.iter().zip(normal).map(|(c, n)| c * n).sum::<i32>() > 0
}
