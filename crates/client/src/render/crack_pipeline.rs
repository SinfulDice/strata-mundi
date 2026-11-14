//! Cracks over the blocks being broken: the crack texture of the stage on the six faces
//! of the block's hit box, a hair outside it, blended over the world.

use super::Gpu;
use super::block_pipeline::BlockPipeline;
use super::depth::DEPTH_FORMAT;
use super::mesh::StreamBuffer;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use strata_core::content::TextureLibrary;
use strata_core::id::Identifier;
use strata_protocol::CRACK_STAGES;

/// Slightly larger than the block so the cracks are not hidden by its faces.
const GROW: f32 = 0.003;
/// Breaks drawn at once, at most (other players' included).
pub const MAX_CRACKS: usize = 32;

/// A block to draw cracked: its hit box relative to the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrackDraw {
    pub min: Vec3,
    pub max: Vec3,
    pub stage: u8,
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
    /// Texture layer of each stage (`strata:crack_0` to `crack_9`), if the packs have it.
    layers: [Option<u32>; CRACK_STAGES as usize],
    count: u32,
}

impl CrackPipeline {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        blocks: &BlockPipeline,
        textures: &TextureLibrary,
    ) -> Self {
        let shader = super::shaders::module(device, "crack.wgsl");
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
            layers: stage_layers(textures),
            count: 0,
        }
    }

    /// Uploads this frame's cracks; call before the render pass begins.
    pub fn prepare(&mut self, gpu: &Gpu, cracks: &[CrackDraw]) {
        let vertices = crack_vertices(cracks, &self.layers);
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

fn stage_layers(textures: &TextureLibrary) -> [Option<u32>; CRACK_STAGES as usize] {
    std::array::from_fn(|stage| {
        let id = Identifier::new("strata", &format!("crack_{stage}")).ok()?;
        textures.layer(&id).map(|layer| u32::from(layer.0))
    })
}

/// Two triangles per face of each crack's box, its stage's texture stretched over the
/// face; stages without a texture are skipped.
fn crack_vertices(cracks: &[CrackDraw], layers: &[Option<u32>]) -> Vec<CrackVertex> {
    let mut vertices = Vec::new();
    for crack in cracks.iter().take(MAX_CRACKS) {
        let Some(&Some(layer)) = layers.get(usize::from(crack.stage)) else {
            continue;
        };
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

    fn crack(stage: u8) -> CrackDraw {
        CrackDraw {
            min: Vec3::ZERO,
            max: Vec3::ONE,
            stage,
        }
    }

    #[test]
    fn each_crack_is_a_box_of_its_stage() {
        let layers: Vec<Option<u32>> = (0..10).map(|stage| Some(100 + stage)).collect();
        let vertices = crack_vertices(&[crack(0), crack(9)], &layers);
        assert_eq!(vertices.len(), 2 * 36);
        assert!(vertices[..36].iter().all(|v| v.layer == 100));
        assert!(vertices[36..].iter().all(|v| v.layer == 109));
        let outside = |v: &CrackVertex| v.position.iter().all(|c| *c < 0.0 || *c > 1.0);
        assert!(vertices.iter().all(outside));
    }

    #[test]
    fn missing_textures_and_stages_draw_nothing() {
        let layers = [None, Some(3)];
        assert!(crack_vertices(&[crack(0), crack(5)], &layers).is_empty());
        let many = vec![crack(1); MAX_CRACKS + 5];
        assert_eq!(crack_vertices(&many, &layers).len(), MAX_CRACKS * 36);
    }
}
