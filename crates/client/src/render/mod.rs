//! wgpu (Vulkan) rendering. Draws into any texture view: a window frame or an off-screen image.

mod block_pipeline;
mod depth;
mod gpu;
mod mesh;
mod offscreen;
mod surface;
mod texture_array;

pub use gpu::{Gpu, GpuError};
pub use mesh::{MeshData, cube};
pub use offscreen::{OFFSCREEN_FORMAT, OffscreenTarget, ReadbackError};
pub use surface::{Acquired, WindowSurface};

use crate::camera::Camera;
use block_pipeline::{BlockPipeline, CameraUniform};
use depth::DepthBuffer;
use glam::DVec3;
use mesh::GpuMesh;
use strata_core::texture::RgbaImage;
use texture_array::TextureArray;

/// Clear color (sRGB) until the sky arrives in v0.2.
const SKY_SRGB: [u8; 3] = [0x78, 0xa7, 0xff];

/// Render targets are sRGB: the clear color must be given in linear space.
fn linear_color([r, g, b]: [u8; 3]) -> wgpu::Color {
    let linear = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    wgpu::Color {
        r: linear(r),
        g: linear(g),
        b: linear(b),
        a: 1.0,
    }
}

/// A mesh placed in the world.
pub struct Placed {
    mesh: GpuMesh,
    origin: DVec3,
}

pub struct Renderer {
    blocks: BlockPipeline,
    depth: DepthBuffer,
    objects: Vec<Placed>,
}

/// Where and how big the frame is.
pub struct Target<'a> {
    pub view: &'a wgpu::TextureView,
    pub size: (u32, u32),
}

impl Renderer {
    pub fn new(gpu: &Gpu, color_format: wgpu::TextureFormat, textures: &[RgbaImage]) -> Self {
        let textures = TextureArray::new(&gpu.device, &gpu.queue, textures);
        Self {
            blocks: BlockPipeline::new(&gpu.device, color_format, &textures),
            depth: DepthBuffer::new(&gpu.device, (1, 1)),
            objects: Vec::new(),
        }
    }

    pub fn add_object(&mut self, gpu: &Gpu, mesh: &MeshData, origin: DVec3) {
        self.objects.push(Placed {
            mesh: GpuMesh::upload(&gpu.device, mesh),
            origin,
        });
    }

    pub fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: Target<'_>,
        camera: &Camera,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        self.depth.ensure_size(&gpu.device, target.size);
        let aspect = target.size.0 as f32 / target.size.1.max(1) as f32;
        self.blocks.write_camera(
            &gpu.queue,
            &CameraUniform {
                view_proj: camera.view_projection(aspect).to_cols_array_2d(),
            },
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(linear_color(SKY_SRGB)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: timestamps,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.blocks.bind(&mut pass);
        for object in &self.objects {
            self.blocks
                .set_mesh_offset(&mut pass, camera.relative(object.origin));
            object.mesh.draw(&mut pass);
        }
    }
}
