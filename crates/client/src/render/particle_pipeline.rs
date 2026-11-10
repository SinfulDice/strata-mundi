//! Draws every particle in one instanced call: a square facing the camera per particle,
//! textured from the block texture array, lit and fogged like blocks.

use super::Gpu;
use super::block_pipeline::BlockPipeline;
use super::depth::DEPTH_FORMAT;
use super::mesh::StreamBuffer;
use crate::camera::Camera;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ParticleInstance {
    /// Centre, relative to the camera.
    pub position: [f32; 3],
    /// Edge, in blocks.
    pub size: f32,
    /// Linear colour factor.
    pub color: [f32; 3],
    pub layer: u32,
    /// Corner of the texture piece and its edge.
    pub uv: [f32; 3],
    /// Sky and block light, 0 to 1.
    pub light: [f32; 2],
}

impl ParticleInstance {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32, 2 => Float32x3, 3 => Uint32, 4 => Float32x3,
            5 => Float32x2
        ],
    };
}

/// Camera right and up vectors (`vec4<f32>` each), to face the squares to the camera.
const BILLBOARD_IMMEDIATE_SIZE: u32 = 32;

pub struct ParticlePipeline {
    /// Particles to draw this frame; filled by the caller, its memory kept between frames.
    pub instances: Vec<ParticleInstance>,
    pipeline: wgpu::RenderPipeline,
    buffer: StreamBuffer,
    count: u32,
}

impl ParticlePipeline {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = super::shaders::module(device, "particle.wgsl");
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle layout"),
            bind_group_layouts: &[Some(camera_layout), Some(texture_layout)],
            immediate_size: BILLBOARD_IMMEDIATE_SIZE,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particles"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(ParticleInstance::LAYOUT)],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
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
            instances: Vec::new(),
            pipeline,
            buffer: StreamBuffer::new("particle instances", wgpu::BufferUsages::VERTEX),
            count: 0,
        }
    }

    /// Uploads this frame's particles; call before the render pass begins.
    pub(super) fn prepare(&mut self, gpu: &Gpu) {
        self.count = self.instances.len() as u32;
        if self.count > 0 {
            self.buffer
                .write(gpu, bytemuck::cast_slice(&self.instances));
        }
    }

    pub(super) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        blocks: &BlockPipeline,
        camera: &Camera,
    ) {
        let Some(buffer) = self.buffer.buffer().filter(|_| self.count > 0) else {
            return;
        };
        let right = camera.flat_right();
        let up = right.cross(camera.forward());
        pass.set_pipeline(&self.pipeline);
        blocks.set_groups(pass);
        pass.set_immediates(0, bytemuck::bytes_of(&right.extend(0.0).to_array()));
        pass.set_immediates(16, bytemuck::bytes_of(&up.extend(0.0).to_array()));
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..4, 0..self.count);
    }
}
