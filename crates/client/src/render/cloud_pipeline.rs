//! The cloud layer: no mesh at all, one instance per cell of a grid that follows the
//! camera, shaped in the vertex shader from a repeating pixel-art pattern. Volumes are
//! drawn in two passes (depth, then color) so their translucent faces never pile up.

use super::depth::DEPTH_FORMAT;
use crate::settings::CloudMode;
use crate::sky::CloudFrame;
use bytemuck::{Pod, Zeroable};
use glam::{DVec2, DVec3, Vec2, Vec3};

/// Cells after which the pattern repeats (as in `clouds.wgsl`).
const PERIOD: u32 = 256;
/// Clouds reach this many times the fog distance, within these bounds (blocks).
const REACH_FACTOR: f32 = 2.0;
const REACH: (f32, f32) = (256.0, 512.0);
/// Vertices of one cell: a single face when flat, a box otherwise.
const FLAT_VERTICES: u32 = 6;
const BOX_VERTICES: u32 = 36;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
struct CloudUniform {
    origin: [f32; 4],
    color: [f32; 4],
    fog: [f32; 4],
    grid: [f32; 4],
}

/// One frame of clouds, worked out on the CPU in `f64` so they never shake far away.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CloudDraw {
    uniform: CloudUniform,
    vertices: u32,
    instances: u32,
}

impl CloudDraw {
    pub(super) fn new(
        clouds: &CloudFrame,
        mode: CloudMode,
        camera: DVec3,
        fog: (Vec3, f32),
    ) -> Option<Self> {
        if mode == CloudMode::Off || clouds.coverage <= 0.0 || clouds.opacity <= 0.0 {
            return None;
        }
        let full = mode == CloudMode::Full;
        let reach = (fog.1 * REACH_FACTOR).clamp(REACH.0, REACH.1);
        let side = (2.0 * reach / clouds.cell_size).ceil() as u32 + 1;
        let origin = pattern_origin(camera, clouds.drift, clouds.cell_size);
        let height = (f64::from(clouds.height) - camera.y) as f32;
        let thickness = if full { clouds.thickness } else { 0.0 };
        Some(Self {
            uniform: CloudUniform {
                origin: [origin.x, origin.y, height, thickness],
                color: clouds.color.extend(clouds.opacity).to_array(),
                fog: fog.0.extend(reach).to_array(),
                grid: [
                    clouds.cell_size,
                    clouds.coverage,
                    side as f32,
                    f32::from(u8::from(full)),
                ],
            },
            vertices: if full { BOX_VERTICES } else { FLAT_VERTICES },
            instances: side * side,
        })
    }
}

/// Where the camera stands in the cloud pattern: wrapped to one period, which is
/// invisible since the pattern repeats, and keeps `f32` precise anywhere in the world.
fn pattern_origin(camera: DVec3, drift: DVec2, cell_size: f32) -> Vec2 {
    let period = f64::from(PERIOD) * f64::from(cell_size);
    let place = DVec2::new(camera.x, camera.z) - drift;
    DVec2::new(place.x.rem_euclid(period), place.y.rem_euclid(period)).as_vec2()
}

pub struct CloudPipeline {
    depth: wgpu::RenderPipeline,
    color: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl CloudPipeline {
    /// Uses the camera (group 0) and block texture (group 1) groups of the block pipeline.
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let cloud_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud layout"),
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
            label: Some("cloud uniform"),
            size: std::mem::size_of::<CloudUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("clouds"),
            layout: &cloud_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cloud pipeline layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(texture_layout),
                Some(&cloud_layout),
            ],
            immediate_size: 0,
        });
        let shader = super::shaders::module(device, "clouds.wgsl");
        Self {
            depth: create_pipeline(device, &layout, &shader, color_format, Pass::Depth),
            color: create_pipeline(device, &layout, &shader, color_format, Pass::Color),
            buffer,
            bind_group,
        }
    }

    pub(super) fn write(&self, queue: &wgpu::Queue, draw: &CloudDraw) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&draw.uniform));
    }

    /// Expects the camera and texture groups to be bound already.
    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, draw: &CloudDraw) {
        pass.set_bind_group(2, &self.bind_group, &[]);
        for pipeline in [&self.depth, &self.color] {
            pass.set_pipeline(pipeline);
            pass.draw(0..draw.vertices, 0..draw.instances);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    /// Only the nearest cloud surface of each pixel, written to the depth buffer.
    Depth,
    /// Blended where the depth pass left that surface.
    Color,
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color_format: wgpu::TextureFormat,
    pass: Pass,
) -> wgpu::RenderPipeline {
    let depth = pass == Pass::Depth;
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if depth { "cloud depth" } else { "clouds" }),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        // Two-sided, so a player flying through the layer still sees it around them.
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth),
            // Reverse-Z, like the blocks.
            depth_compare: Some(if depth {
                wgpu::CompareFunction::GreaterEqual
            } else {
                wgpu::CompareFunction::Equal
            }),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: if depth {
                    wgpu::ColorWrites::empty()
                } else {
                    wgpu::ColorWrites::ALL
                },
            })],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn frame(drift: DVec2) -> CloudFrame {
        CloudFrame {
            color: Vec3::ONE,
            opacity: 0.8,
            height: 300.0,
            thickness: 4.0,
            cell_size: 12.0,
            coverage: 0.4,
            drift,
        }
    }

    fn fog() -> (Vec3, f32) {
        (Vec3::ZERO, 100.0)
    }

    #[test]
    fn the_pattern_origin_moves_smoothly_and_wraps_unseen() {
        let period = f64::from(PERIOD) * 12.0;
        let camera = DVec3::new(1.0e7 + 0.3, 120.0, -4.0e6);
        let mut previous = pattern_origin(camera, DVec2::ZERO, 12.0);
        for step in 1..=20_000 {
            let drift = DVec2::new(f64::from(step) * 0.05, 0.0);
            let now = pattern_origin(camera, drift, 12.0);
            let moved = (previous.x - now.x).abs();
            let wrapped = (moved - period as f32).abs();
            assert!(moved < 0.06 || wrapped < 0.06, "jump at step {step}");
            assert!(now.x >= 0.0 && now.x <= period as f32);
            previous = now;
        }
        let far = pattern_origin(camera + DVec3::new(period, 0.0, 0.0), DVec2::ZERO, 12.0);
        assert!((far - pattern_origin(camera, DVec2::ZERO, 12.0)).length() < 1e-3);
    }

    #[test]
    fn modes_choose_the_geometry() {
        let clouds = frame(DVec2::ZERO);
        let camera = DVec3::new(5.0, 100.0, 5.0);
        assert!(CloudDraw::new(&clouds, CloudMode::Off, camera, fog()).is_none());
        let flat = CloudDraw::new(&clouds, CloudMode::Flat, camera, fog()).unwrap();
        let full = CloudDraw::new(&clouds, CloudMode::Full, camera, fog()).unwrap();
        assert_eq!(
            (flat.vertices, full.vertices),
            (FLAT_VERTICES, BOX_VERTICES)
        );
        assert_eq!((flat.uniform.origin[3], full.uniform.origin[3]), (0.0, 4.0));
        assert_eq!(
            flat.uniform.origin[2], 200.0,
            "height relative to the camera"
        );
        assert_eq!(flat.instances, flat.uniform.grid[2].powi(2) as u32);
        let clear = CloudFrame {
            coverage: 0.0,
            ..clouds
        };
        assert!(CloudDraw::new(&clear, CloudMode::Full, camera, fog()).is_none());
    }

    #[test]
    fn the_reach_follows_the_fog_within_bounds() {
        let clouds = frame(DVec2::ZERO);
        let reach = |end| {
            CloudDraw::new(&clouds, CloudMode::Flat, DVec3::ZERO, (Vec3::ZERO, end))
                .unwrap()
                .uniform
                .fog[3]
        };
        assert_eq!(reach(50.0), REACH.0);
        assert_eq!(reach(200.0), 400.0);
        assert_eq!(reach(f32::MAX), REACH.1);
    }
}
