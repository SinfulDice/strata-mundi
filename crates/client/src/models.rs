//! Entity models: boxes from the pack turned into textured quads once, then placed,
//! turned and moved around the camera every frame.

use bytemuck::{Pod, Zeroable};
use glam::{DVec3, Vec2, Vec3};
use strata_core::content::{Content, EntityTypeId, ModelBox};
use strata_core::light::{MAX_LIGHT, PackedLight};

/// Light of each face (up, down, north, south, east, west), like blocks.
const SHADES: [f32; 6] = [1.0, 0.5, 0.8, 0.8, 0.6, 0.6];
const SIXTEENTH: f32 = 1.0 / 16.0;

/// A model vertex, relative to the camera.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub layer: u32,
    pub shade: f32,
    /// Sky and block light of the entity's cell, 0 to 1.
    pub light: [f32; 2],
}

impl ModelVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Uint32, 3 => Float32, 4 => Float32x2],
    };
}

/// Where an entity is drawn this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPose {
    pub position: DVec3,
    /// Radians, 0 = north (-Z).
    pub yaw: f32,
}

#[derive(Debug, Clone, Copy)]
struct Corner {
    position: Vec3,
    uv: Vec2,
    shade: f32,
}

#[derive(Debug, Clone, Default)]
struct ModelMesh {
    /// Two triangles per quad, in model space (blocks).
    corners: Vec<Corner>,
    layer: u32,
    /// Farthest corner from the origin, for culling whatever the yaw.
    radius: f32,
}

/// The models of every entity type, by `EntityTypeId`.
#[derive(Debug, Clone, Default)]
pub struct ModelLibrary {
    meshes: Vec<ModelMesh>,
}

impl ModelLibrary {
    pub fn from_content(content: &Content) -> Self {
        let meshes = content
            .entities
            .iter()
            .map(|(_, _, entity)| mesh(&entity.boxes, u32::from(entity.texture.0)))
            .collect();
        Self { meshes }
    }

    /// Radius of the sphere around the origin that holds the model.
    pub fn radius(&self, kind: EntityTypeId) -> Option<f32> {
        self.meshes.get(usize::from(kind.0)).map(|mesh| mesh.radius)
    }

    /// Appends the triangles of `kind` at `pose`, lit by `light`, relative to `camera`.
    pub fn place(
        &self,
        (kind, pose, light): (EntityTypeId, ModelPose, PackedLight),
        camera: DVec3,
        out: &mut Vec<ModelVertex>,
    ) {
        let max = f32::from(MAX_LIGHT);
        let light = [light.sky(), light.block()].map(|level| f32::from(level) / max);
        let Some(mesh) = self.meshes.get(usize::from(kind.0)) else {
            return;
        };
        let origin = (pose.position - camera).as_vec3();
        let (sin, cos) = pose.yaw.sin_cos();
        out.extend(mesh.corners.iter().map(|corner| {
            let p = corner.position;
            // Turns the model's front (-Z) towards the yaw: (0, -1) → (sin, -cos).
            let turned = Vec3::new(p.x * cos - p.z * sin, p.y, p.x * sin + p.z * cos);
            ModelVertex {
                position: (origin + turned).to_array(),
                uv: corner.uv.to_array(),
                layer: mesh.layer,
                shade: corner.shade,
                light,
            }
        }));
    }
}

fn mesh(boxes: &[ModelBox], layer: u32) -> ModelMesh {
    let mut corners = Vec::with_capacity(boxes.len() * 36);
    for model_box in boxes {
        let min = Vec3::new(
            model_box.from.0 as f32,
            model_box.from.1 as f32,
            model_box.from.2 as f32,
        ) * SIXTEENTH;
        let max = Vec3::new(
            model_box.to.0 as f32,
            model_box.to.1 as f32,
            model_box.to.2 as f32,
        ) * SIXTEENTH;
        for (face, quad) in box_faces(min, max).into_iter().enumerate() {
            let shade = SHADES[face];
            let corner = |i: usize| Corner {
                position: quad[i].0,
                uv: quad[i].1,
                shade,
            };
            corners.extend([0, 1, 2, 0, 2, 3].map(corner));
        }
    }
    let radius = corners
        .iter()
        .map(|c| c.position.length())
        .fold(0.0, f32::max);
    ModelMesh {
        corners,
        layer,
        radius,
    }
}

/// The six faces of a box (in `Face` order), each as four corners with texture
/// coordinates: one texture repeat per block, `v` growing downwards on the sides.
fn box_faces(a: Vec3, b: Vec3) -> [[(Vec3, Vec2); 4]; 6] {
    let side = |x: f32, y: f32, z: f32, u: f32| (Vec3::new(x, y, z), Vec2::new(u, -y));
    [
        flat(a, b, b.y),
        flat(a, b, a.y),
        [
            side(a.x, a.y, a.z, -a.x),
            side(b.x, a.y, a.z, -b.x),
            side(b.x, b.y, a.z, -b.x),
            side(a.x, b.y, a.z, -a.x),
        ],
        [
            side(a.x, a.y, b.z, a.x),
            side(b.x, a.y, b.z, b.x),
            side(b.x, b.y, b.z, b.x),
            side(a.x, b.y, b.z, a.x),
        ],
        [
            side(b.x, a.y, a.z, a.z),
            side(b.x, a.y, b.z, b.z),
            side(b.x, b.y, b.z, b.z),
            side(b.x, b.y, a.z, a.z),
        ],
        [
            side(a.x, a.y, a.z, -a.z),
            side(a.x, a.y, b.z, -b.z),
            side(a.x, b.y, b.z, -b.z),
            side(a.x, b.y, a.z, -a.z),
        ],
    ]
}

/// A horizontal face at height `y`, textured by its x and z.
fn flat(a: Vec3, b: Vec3, y: f32) -> [(Vec3, Vec2); 4] {
    [(a.x, a.z), (b.x, a.z), (b.x, b.z), (a.x, b.z)]
        .map(|(x, z)| (Vec3::new(x, y, z), Vec2::new(x, z)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> ModelLibrary {
        let boxes = [ModelBox {
            from: (-8, 0, -16),
            to: (8, 8, 0),
        }];
        ModelLibrary {
            meshes: vec![mesh(&boxes, 3)],
        }
    }

    #[test]
    fn a_box_gives_six_quads_and_turns_with_the_yaw() {
        let models = library();
        let pose = ModelPose {
            position: DVec3::new(10.0, 64.0, 10.0),
            yaw: std::f32::consts::FRAC_PI_2,
        };
        let mut out = Vec::new();
        models.place(
            (EntityTypeId(0), pose, PackedLight::new(15, 3)),
            DVec3::new(10.0, 60.0, 10.0),
            &mut out,
        );
        assert_eq!(out.len(), 36);
        let farthest_east = out.iter().map(|v| v.position[0]).fold(f32::MIN, f32::max);
        assert!(
            (farthest_east - 1.0).abs() < 1e-5,
            "the front now points east"
        );
        assert!(out.iter().all(|v| v.position[1] >= 4.0 && v.layer == 3));
        assert!(out.iter().all(|v| v.light == [1.0, 0.2]));
        assert!(models.radius(EntityTypeId(0)).is_some_and(|r| r > 1.0));
        assert!(models.radius(EntityTypeId(1)).is_none());
    }
}
