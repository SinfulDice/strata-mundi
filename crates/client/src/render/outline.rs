use super::depth::DEPTH_FORMAT;
use glam::Vec3;

/// The 12 edges of the aimed block, drawn as thin dark lines over the world.
pub struct OutlinePipeline {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
}

/// Slightly larger than the block so the lines are not hidden by its faces.
const GROW: f32 = 0.002;
const VERTEX_COUNT: u32 = 24;

impl OutlinePipeline {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = super::shaders::module(device, "outline.wgsl");
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("outline layout"),
            bind_group_layouts: &[Some(camera_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("outline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
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
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("outline vertices"),
            size: u64::from(VERTEX_COUNT) * 12,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { pipeline, vertices }
    }

    /// `min` is the block corner relative to the camera. The camera bind group must be set.
    pub fn draw(&self, queue: &wgpu::Queue, pass: &mut wgpu::RenderPass<'_>, min: Vec3) {
        let (a, b) = (min - GROW, min + 1.0 + GROW);
        let corner = |i: u32| {
            Vec3::new(
                if i & 1 == 0 { a.x } else { b.x },
                if i & 2 == 0 { a.y } else { b.y },
                if i & 4 == 0 { a.z } else { b.z },
            )
        };
        let edges = (0..8u32)
            .flat_map(|i| [1, 2, 4].map(move |bit| (i, i | bit)))
            .filter(|(i, j)| i != j);
        let points: Vec<[f32; 3]> = edges
            .flat_map(|(i, j)| [corner(i).to_array(), corner(j).to_array()])
            .collect();
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&points));
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..VERTEX_COUNT, 0..1);
    }
}
