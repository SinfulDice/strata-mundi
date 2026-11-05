//! The sky behind the world: one screen-covering triangle on the far plane, with the
//! gradient, the glow of sunrise and sunset, the sun, the moon and the stars.

use super::depth::DEPTH_FORMAT;
use crate::sky::SkyFrame;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SkyUniform {
    inverse_view_proj: [[f32; 4]; 4],
    /// w: glow strength.
    zenith: [f32; 4],
    /// w: star visibility.
    horizon: [f32; 4],
    /// w: share of the sky's cells holding a star.
    glow: [f32; 4],
    /// w: sun half-width (tangent).
    sun: [f32; 4],
    /// w: moon half-width (tangent).
    across: [f32; 4],
    /// Sun layer, moon layer, moon phase.
    bodies: [u32; 4],
    /// x: twinkle; y: star time (seconds wrapped on a turn).
    stars: [f32; 4],
}

pub struct SkyPipeline {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl SkyPipeline {
    /// Uses the camera (group 0) and block textures (group 1) of the block pipeline.
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let sky_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky uniform"),
            size: std::mem::size_of::<SkyUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky"),
            layout: &sky_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky pipeline layout"),
            bind_group_layouts: &[Some(camera_layout), Some(texture_layout), Some(&sky_layout)],
            immediate_size: 0,
        });
        Self {
            pipeline: create_pipeline(device, &layout, color_format),
            buffer,
            bind_group,
        }
    }

    pub fn write(
        &self,
        queue: &wgpu::Queue,
        sky: &SkyFrame,
        view_projection: Mat4,
        star_time: f32,
    ) {
        let uniform = SkyUniform {
            inverse_view_proj: view_projection.inverse().to_cols_array_2d(),
            zenith: sky.zenith.extend(sky.glow_strength).to_array(),
            horizon: sky.horizon.extend(sky.stars).to_array(),
            glow: sky.glow.extend(sky.star_density).to_array(),
            sun: sky.sun.extend(sky.sun_size).to_array(),
            across: sky.across.extend(sky.moon_size).to_array(),
            bodies: [sky.sun_layer, sky.moon_layer, sky.moon_phase, 0],
            stars: [sky.star_twinkle, star_time, 0.0, 0.0],
        };
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&uniform));
    }

    /// Expects the camera and texture groups to be bound already.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = super::shaders::module(device, "sky.wgsl");
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sky"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        primitive: Default::default(),
        // On the far plane (0 in reverse-Z), after the opaque world: only the pixels it
        // left empty are shaded. The depth buffer is left untouched.
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
            targets: &[Some(color_format.into())],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
