use super::depth::DEPTH_FORMAT;
use super::texture_array::TextureArray;
use crate::mesher::PackedVertex;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    /// x: ambient occlusion strength.
    pub options: [f32; 4],
}

pub const SECTION_OFFSET_SIZE: u32 = 16;

/// The two passes of block rendering share bindings: opaque/cutout first, then translucent.
pub struct BlockPipeline {
    cutout: wgpu::RenderPipeline,
    translucent: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_bind_group: wgpu::BindGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockPass {
    Cutout,
    Translucent,
}

impl BlockPipeline {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        textures: &TextureArray,
    ) -> Self {
        let camera_layout = uniform_layout(device);
        let texture_layout = texture_layout(device);
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("block textures"),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&textures.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&textures.sampler),
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("block pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
            immediate_size: SECTION_OFFSET_SIZE,
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/block.wgsl"));
        Self {
            cutout: create_pipeline(device, &layout, &shader, color_format, BlockPass::Cutout),
            translucent: create_pipeline(
                device,
                &layout,
                &shader,
                color_format,
                BlockPass::Translucent,
            ),
            camera_buffer,
            camera_bind_group,
            texture_bind_group,
        }
    }

    pub fn write_camera(&self, queue: &wgpu::Queue, camera: &CameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(camera));
    }

    pub fn bind(&self, pass: &mut wgpu::RenderPass<'_>, which: BlockPass) {
        pass.set_pipeline(match which {
            BlockPass::Cutout => &self.cutout,
            BlockPass::Translucent => &self.translucent,
        });
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.texture_bind_group, &[]);
    }

    /// `offset` is the section origin relative to the camera.
    pub fn set_section_offset(pass: &mut wgpu::RenderPass<'_>, offset: glam::Vec3) {
        pass.set_immediates(0, bytemuck::bytes_of(&offset.extend(0.0).to_array()));
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    which: BlockPass,
) -> wgpu::RenderPipeline {
    let translucent = which == BlockPass::Translucent;
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if translucent {
            "translucent blocks"
        } else {
            "cutout blocks"
        }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(PackedVertex::LAYOUT)],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            // Translucent surfaces are tested against the depth buffer but do not write it,
            // so water does not hide what is behind other water.
            depth_write_enabled: Some(!translucent),
            // Reverse-Z: far = 0, near = 1, for precision over long view distances.
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(if translucent {
                "fs_translucent"
            } else {
                "fs_cutout"
            }),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: translucent.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn uniform_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("camera layout"),
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
    })
}

fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("block texture layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}
