//! Draws entity models with the block textures and camera, in one draw call per frame.

use super::Gpu;
use super::block_pipeline::BlockPipeline;
use super::depth::DEPTH_FORMAT;
use super::frustum::Frustum;
use super::mesh::StreamBuffer;
use crate::camera::Camera;
use crate::models::{ModelLibrary, ModelPose, ModelVertex};
use strata_core::content::EntityTypeId;
use strata_core::light::PackedLight;

pub struct ModelPipeline {
    pub library: ModelLibrary,
    /// Entities to draw this frame, with the light of their cell.
    pub entities: Vec<(EntityTypeId, ModelPose, PackedLight)>,
    pipeline: wgpu::RenderPipeline,
    vertices: StreamBuffer,
    scratch: Vec<ModelVertex>,
    count: u32,
}

impl ModelPipeline {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = super::shaders::module(device, "model.wgsl");
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model layout"),
            bind_group_layouts: &[Some(camera_layout), Some(texture_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("models"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(ModelVertex::LAYOUT)],
                compilation_options: Default::default(),
            },
            // Faces of both windings are drawn: models are few and tiny.
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
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
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            library: ModelLibrary::default(),
            entities: Vec::new(),
            pipeline,
            vertices: StreamBuffer::new("model vertices", wgpu::BufferUsages::VERTEX),
            scratch: Vec::new(),
            count: 0,
        }
    }

    /// Places the visible models around the camera and uploads them; call before the
    /// render pass begins.
    pub(super) fn prepare(&mut self, gpu: &Gpu, camera: &Camera, frustum: &Frustum) {
        self.scratch.clear();
        for &(kind, pose, light) in &self.entities {
            let Some(radius) = self.library.radius(kind) else {
                continue;
            };
            let centre = camera.relative(pose.position);
            if frustum.intersects_box(centre - radius, centre + radius) {
                self.library
                    .place((kind, pose, light), camera.position, &mut self.scratch);
            }
        }
        self.count = self.scratch.len() as u32;
        if self.count > 0 {
            self.vertices
                .write(gpu, bytemuck::cast_slice(&self.scratch));
        }
    }

    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, blocks: &BlockPipeline) {
        let Some(buffer) = self.vertices.buffer().filter(|_| self.count > 0) else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        blocks.set_groups(pass);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}
