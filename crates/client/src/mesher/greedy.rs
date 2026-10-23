//! Section mesher: face culling, greedy merging of identical neighbouring faces, and
//! per-vertex ambient occlusion. Pure CPU code, run on worker threads.

use super::neighborhood::{BlockTable, Neighborhood, SIZE};
use super::vertex::{MeshData, PackedVertex};
use strata_core::content::{Axis, Face, RenderKind};

type Vec3i = [i32; 3];

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
struct FaceFrame {
    normal: Vec3i,
    a: Vec3i,
    b: Vec3i,
    right: Vec3i,
    down: Vec3i,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FaceKey {
    layer: u16,
    turned: bool,
    ao: [u8; 4],
    translucent: bool,
}

/// With `ambient_occlusion` off, every corner is fully lit, which also lets more faces merge.
pub fn mesh_section(
    blocks: &Neighborhood,
    table: &BlockTable,
    ambient_occlusion: bool,
) -> SectionMesh {
    let mut mesh = SectionMesh::default();
    for face in Face::ALL {
        let frame = frame(face);
        for depth in 0..SIZE as i32 {
            let mut mask = [[None; SIZE]; SIZE];
            for (b, row) in mask.iter_mut().enumerate() {
                for (a, cell) in row.iter_mut().enumerate() {
                    let pos = cell_position(&frame, depth, a as i32, b as i32);
                    *cell = face_key(blocks, table, face, &frame, pos, ambient_occlusion);
                }
            }
            merge_slice(&mut mask, |rect, key| {
                let target = if key.translucent {
                    &mut mesh.translucent
                } else {
                    &mut mesh.opaque
                };
                emit_quad(target, face, &frame, depth, rect, key);
            });
        }
    }
    mesh
}

/// Block position for cell `(a, b)` of slice `depth` along the face normal.
fn cell_position(frame: &FaceFrame, depth: i32, a: i32, b: i32) -> Vec3i {
    let axis = |v: Vec3i| v.map(i32::abs);
    let n = axis(frame.normal);
    std::array::from_fn(|i| n[i] * depth + frame.a[i] * a + frame.b[i] * b)
}

fn add(p: Vec3i, q: Vec3i) -> Vec3i {
    [p[0] + q[0], p[1] + q[1], p[2] + q[2]]
}

fn scale(v: Vec3i, k: i32) -> Vec3i {
    v.map(|c| c * k)
}

fn face_key(
    blocks: &Neighborhood,
    table: &BlockTable,
    face: Face,
    frame: &FaceFrame,
    pos: Vec3i,
    ambient_occlusion: bool,
) -> Option<FaceKey> {
    let id = blocks.get(pos[0], pos[1], pos[2]);
    let block = table.get(id);
    if block.render == RenderKind::Invisible {
        return None;
    }
    let front = add(pos, frame.normal);
    let neighbour = blocks.get(front[0], front[1], front[2]);
    if table.is_opaque(neighbour) || (neighbour == id && block.cull_same) {
        return None;
    }
    let occludes = |p: Vec3i| ambient_occlusion && table.is_opaque(blocks.get(p[0], p[1], p[2]));
    let ao = [(-1, -1), (1, -1), (1, 1), (-1, 1)].map(|(da, db)| {
        let side_a = add(front, scale(frame.a, da));
        let side_b = add(front, scale(frame.b, db));
        let corner = add(side_a, scale(frame.b, db));
        vertex_ao(occludes(side_a), occludes(side_b), occludes(corner))
    });
    Some(FaceKey {
        layer: block.faces[face.index()],
        turned: block.turned[face.index()],
        ao,
        translucent: block.render == RenderKind::Translucent,
    })
}

/// 3 = fully lit, 0 = darkest corner (both sides blocked).
fn vertex_ao(side_a: bool, side_b: bool, corner: bool) -> u8 {
    if side_a && side_b {
        0
    } else {
        3 - u8::from(side_a) - u8::from(side_b) - u8::from(corner)
    }
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
            let width = (a..SIZE).take_while(|&i| mask[b][i] == Some(key)).count();
            let height = (b..SIZE)
                .take_while(|&j| (a..a + width).all(|i| mask[j][i] == Some(key)))
                .count();
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
    face: Face,
    frame: &FaceFrame,
    depth: i32,
    rect: Rect,
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
        (corner(a0, b0), key.ao[0]),
        (corner(a0 + w, b0), key.ao[1]),
        (corner(a0 + w, b0 + h), key.ao[2]),
        (corner(a0, b0 + h), key.ao[3]),
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
    let vertices = corners.map(|(p, ao)| {
        let (u, v) = (dot(p, frame.right) - min_u, dot(p, frame.down) - min_v);
        let uv = if key.turned { [v, width - u] } else { [u, v] }.map(|c| c as u32);
        PackedVertex::new(p.map(|c| c as u32), face.index(), ao, uv, key.layer)
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
