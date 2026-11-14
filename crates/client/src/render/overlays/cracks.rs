//! Cracks over the blocks being broken: the crack texture of the stage on the six faces
//! of the block's hit box, a hair outside it, blended over the world.

use crate::render::Gpu;
use crate::render::block_pipeline::BlockPipeline;
use crate::render::depth::DEPTH_FORMAT;
use crate::render::mesh::StreamBuffer;
use bytemuck::{Pod, Zeroable};
use glam::{DVec3, Vec3};
use strata_core::content::HitBox;
use strata_core::coords::BlockPos;

/// Slightly larger than the block so the cracks are not hidden by its faces.
const GROW: f32 = 0.003;
/// Breaks drawn at once, at most (other players' included).
pub const MAX_CRACKS: usize = 32;

/// A block to draw cracked: its hit box relative to the camera, and the texture layer
/// of its crack stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrackDraw {
    pub min: Vec3,
    pub max: Vec3,
    pub layer: u32,
}

impl CrackDraw {
    /// Cracks drawn with texture `layer` on the block at `pos` whose aim stops on
    /// `shape`, seen from `eyes`.
    pub fn on(pos: BlockPos, shape: HitBox, layer: u32, eyes: DVec3) -> Self {
        let corner = DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z));
        Self {
            min: (corner + DVec3::from(shape.min) - eyes).as_vec3(),
            max: (corner + DVec3::from(shape.max) - eyes).as_vec3(),
            layer,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
struct CrackVertex {
    position: [f32; 3],
    uv: [f32; 2],
    layer: u32,
}

pub struct CrackPipeline {
    pipeline: wgpu::RenderPipeline,
    vertices: StreamBuffer,
    count: u32,
}

impl CrackPipeline {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        blocks: &BlockPipeline,
    ) -> Self {
        let shader = crate::render::shaders::module(device, "crack.wgsl");
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("crack layout"),
            bind_group_layouts: &[Some(&blocks.camera_layout), Some(&blocks.texture_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cracks"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<CrackVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Uint32],
                })],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            vertices: StreamBuffer::new("crack vertices", wgpu::BufferUsages::VERTEX),
            count: 0,
        }
    }

    /// Uploads this frame's cracks; call before the render pass begins.
    pub fn prepare(&mut self, gpu: &Gpu, cracks: &[CrackDraw]) {
        let vertices = crack_vertices(cracks);
        self.count = vertices.len() as u32;
        if self.count > 0 {
            self.vertices.write(gpu, bytemuck::cast_slice(&vertices));
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, blocks: &BlockPipeline) {
        let Some(buffer) = self.vertices.buffer().filter(|_| self.count > 0) else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        blocks.set_groups(pass);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

/// Two triangles per face of each crack's box, its stage's texture stretched over the
/// face.
fn crack_vertices(cracks: &[CrackDraw]) -> Vec<CrackVertex> {
    let mut vertices = Vec::new();
    for crack in cracks.iter().take(MAX_CRACKS) {
        let layer = crack.layer;
        let (a, b) = (crack.min - GROW, crack.max + GROW);
        for face in FACES {
            let corner = |i: usize| {
                let [x, y, z] = face[i];
                Vec3::new(pick(x, a.x, b.x), pick(y, a.y, b.y), pick(z, a.z, b.z))
            };
            for i in [0, 1, 2, 0, 2, 3] {
                vertices.push(CrackVertex {
                    position: corner(i).to_array(),
                    uv: UVS[i],
                    layer,
                });
            }
        }
    }
    vertices
}

fn pick(one: u8, low: f32, high: f32) -> f32 {
    if one == 1 { high } else { low }
}

/// Corners of each face of the unit cube (1 = the high side), going round.
const FACES: [[[u8; 3]; 4]; 6] = [
    [[0, 1, 0], [1, 1, 0], [1, 1, 1], [0, 1, 1]],
    [[0, 0, 1], [1, 0, 1], [1, 0, 0], [0, 0, 0]],
    [[1, 1, 0], [0, 1, 0], [0, 0, 0], [1, 0, 0]],
    [[0, 1, 1], [1, 1, 1], [1, 0, 1], [0, 0, 1]],
    [[1, 1, 1], [1, 1, 0], [1, 0, 0], [1, 0, 1]],
    [[0, 1, 0], [0, 1, 1], [0, 0, 1], [0, 0, 0]],
];
const UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

#[cfg(test)]
mod tests {
    use super::*;

    fn crack(layer: u32) -> CrackDraw {
        CrackDraw {
            min: Vec3::ZERO,
            max: Vec3::ONE,
            layer,
        }
    }

    #[test]
    fn each_crack_is_a_box_of_its_stage() {
        let vertices = crack_vertices(&[crack(100), crack(109)]);
        assert_eq!(vertices.len(), 2 * 36);
        assert!(vertices[..36].iter().all(|v| v.layer == 100));
        assert!(vertices[36..].iter().all(|v| v.layer == 109));
        let outside = |v: &CrackVertex| v.position.iter().all(|c| *c < 0.0 || *c > 1.0);
        assert!(vertices.iter().all(outside));
    }

    #[test]
    fn the_number_of_cracks_is_bounded() {
        let many = vec![crack(1); MAX_CRACKS + 5];
        assert_eq!(crack_vertices(&many).len(), MAX_CRACKS * 36);
    }
}
