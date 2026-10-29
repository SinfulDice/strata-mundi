//! wgpu (Vulkan) rendering. Draws into any texture view: a window frame or an off-screen image.

mod block_pipeline;
mod depth;
mod frustum;
mod gpu;
mod mesh;
mod offscreen;
mod outline;
mod post;
mod shaders;
mod surface;
mod texture_array;
mod ui_pipeline;

pub use gpu::{Gpu, GpuError};
pub use offscreen::{OFFSCREEN_FORMAT, OffscreenTarget, ReadbackError};
pub use post::PostProcess;
pub use surface::{Acquired, WindowSurface};
pub use ui_pipeline::UiPipeline;

use crate::camera::Camera;
use crate::mesher::{SectionKey, SectionMesh};
use crate::settings::{ColorFilter, GraphicsSettings, Settings};
use block_pipeline::{BlockPass, BlockPipeline, CameraUniform};
use depth::DepthBuffer;
use frustum::Frustum;
pub use frustum::in_vertical_range;
use glam::DVec3;
use mesh::GpuMesh;
use outline::OutlinePipeline;
use std::collections::HashMap;
use strata_core::content::TextureLibrary;
use strata_core::coords::BlockPos;
use strata_core::coords::{SECTION_SIZE, SECTIONS_PER_COLUMN};
use texture_array::TextureArray;
pub use texture_array::TextureFiltering;

/// Clear color (sRGB) until the sky arrives in v0.2.
const SKY_SRGB: [u8; 3] = [0x78, 0xa7, 0xff];

/// Fog starts at this fraction of the render distance.
const FOG_START: f32 = 0.6;

fn srgb_to_linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Render targets are sRGB: the clear color must be given in linear space.
fn linear_color([r, g, b]: [u8; 3]) -> wgpu::Color {
    wgpu::Color {
        r: srgb_to_linear(r),
        g: srgb_to_linear(g),
        b: srgb_to_linear(b),
        a: 1.0,
    }
}

fn linear_rgba([r, g, b]: [u8; 3]) -> [f32; 4] {
    let [r, g, b] = [r, g, b].map(|c| srgb_to_linear(c) as f32);
    [r, g, b, 1.0]
}

/// GPU meshes of one section.
struct GpuSection {
    origin: DVec3,
    cutout: Option<GpuMesh>,
    translucent: Option<GpuMesh>,
}

pub struct Renderer {
    blocks: BlockPipeline,
    outline_pipeline: OutlinePipeline,
    /// Block drawn with an outline (the one the player aims at).
    pub outline: Option<BlockPos>,
    depth: DepthBuffer,
    /// Section meshes grouped by column, so unloading a column is a single removal.
    columns: HashMap<(i32, i32), Vec<Option<GpuSection>>>,
    /// Ambient occlusion strength, 0 (off) to 1.
    pub ambient_occlusion: f32,
    /// Sections farther than this above or below the camera's section are not drawn.
    pub vertical_distance: u32,
    pub options: RenderOptions,
}

/// Image settings that need no rebuild, taken from the player's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    pub fog: bool,
    /// Where the fog is complete, in blocks (the edge of the loaded world).
    pub fog_end: f32,
    /// 3D resolution, as a fraction of the window.
    pub render_scale: f32,
    pub color_filter: ColorFilter,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            fog: false,
            fog_end: f32::MAX,
            render_scale: 1.0,
            color_filter: ColorFilter::None,
        }
    }
}

impl RenderOptions {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            render_scale: settings.graphics.render_scale as f32 / 100.0,
            color_filter: settings.interface.color_filter,
            ..Self::for_capture(&settings.graphics)
        }
    }

    /// Captures: the fog of the graphics settings, full resolution, no filter.
    pub fn for_capture(graphics: &GraphicsSettings) -> Self {
        Self {
            fog: graphics.fog,
            fog_end: (graphics.render_distance as f32 - 0.5) * SECTION_SIZE as f32,
            render_scale: 1.0,
            color_filter: ColorFilter::None,
        }
    }

    /// Whether the image goes through the final pass (scaling, colour filter).
    pub fn needs_post(&self) -> bool {
        (self.render_scale - 1.0).abs() > f32::EPSILON || self.color_filter != ColorFilter::None
    }
}

/// Where and how big the frame is.
pub struct Target<'a> {
    pub view: &'a wgpu::TextureView,
    pub size: (u32, u32),
}

impl Renderer {
    pub fn new(
        gpu: &Gpu,
        color_format: wgpu::TextureFormat,
        textures: &TextureLibrary,
        filtering: TextureFiltering,
    ) -> Result<Self, GpuError> {
        let textures = texture_array(gpu, textures, filtering)?;
        let blocks = BlockPipeline::new(&gpu.device, color_format, &textures);
        Ok(Self {
            outline_pipeline: OutlinePipeline::new(
                &gpu.device,
                color_format,
                &blocks.camera_layout,
            ),
            outline: None,
            blocks,
            depth: DepthBuffer::new(&gpu.device, (1, 1)),
            columns: HashMap::new(),
            ambient_occlusion: 1.0,
            vertical_distance: u32::MAX,
            options: RenderOptions::default(),
        })
    }

    /// Rebuilds the block textures with new mipmap and anisotropy settings.
    pub fn set_texture_filtering(
        &mut self,
        gpu: &Gpu,
        textures: &TextureLibrary,
        mipmaps: bool,
        anisotropy: u32,
    ) {
        let filtering = TextureFiltering {
            mipmaps,
            anisotropy: anisotropy.clamp(1, 16) as u16,
        };
        match texture_array(gpu, textures, filtering) {
            Ok(array) => self.blocks.set_textures(&gpu.device, &array),
            Err(error) => tracing::error!(%error, "texture settings not applied"),
        }
    }

    /// Replaces the meshes of a section (an empty mesh removes it).
    pub fn upload_section(&mut self, gpu: &Gpu, key: SectionKey, mesh: &SectionMesh) {
        let slots = self
            .columns
            .entry((key.0, key.1))
            .or_insert_with(|| (0..SECTIONS_PER_COLUMN).map(|_| None).collect());
        let Some(slot) = slots.get_mut(key.2) else {
            return;
        };
        if mesh.is_empty() {
            *slot = None;
            return;
        }
        let origin = DVec3::new(
            f64::from(key.0 * SECTION_SIZE),
            (key.2 as i32 * SECTION_SIZE).into(),
            f64::from(key.1 * SECTION_SIZE),
        );
        let section = GpuSection {
            origin,
            cutout: GpuMesh::upload(&gpu.device, &mesh.opaque),
            translucent: GpuMesh::upload(&gpu.device, &mesh.translucent),
        };
        *slot = Some(section);
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
        let view_projection = camera.view_projection(aspect);
        let frustum = Frustum::from_view_projection(view_projection);
        self.blocks.write_camera(
            &gpu.queue,
            &CameraUniform {
                view_proj: view_projection.to_cols_array_2d(),
                options: [
                    self.ambient_occlusion,
                    self.options.fog_end * FOG_START,
                    self.options.fog_end,
                    f32::from(u8::from(self.options.fog)),
                ],
                sky: linear_rgba(SKY_SRGB),
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
        self.blocks.bind(&mut pass, BlockPass::Cutout);
        for section in self.visible_sections(camera, &frustum) {
            if let Some(mesh) = &section.cutout {
                BlockPipeline::set_section_offset(&mut pass, camera.relative(section.origin));
                mesh.draw(&mut pass);
            }
        }
        if let Some(block) = self.outline {
            let corner = DVec3::new(f64::from(block.x), f64::from(block.y), f64::from(block.z));
            self.outline_pipeline
                .draw(&gpu.queue, &mut pass, camera.relative(corner));
        }
        self.draw_translucent(&mut pass, camera, &frustum);
    }

    /// Within the vertical distance and inside the view frustum.
    fn visible_sections<'a>(
        &'a self,
        camera: &'a Camera,
        frustum: &'a Frustum,
    ) -> impl Iterator<Item = &'a GpuSection> {
        let camera_section = (camera.position.y / f64::from(SECTION_SIZE)).floor() as i64;
        let distance = self.vertical_distance;
        self.columns.values().flat_map(move |slots| {
            let top = slots.iter().rposition(Option::is_some).unwrap_or(0);
            slots.iter().enumerate().filter_map(move |(index, slot)| {
                let section = slot.as_ref()?;
                let min = camera.relative(section.origin);
                let visible = in_vertical_range(index, top, camera_section, distance)
                    && frustum.intersects_box(min, min + glam::Vec3::splat(SECTION_SIZE as f32));
                visible.then_some(section)
            })
        })
    }

    /// Drops every section of a column that left the view distance.
    pub fn remove_column(&mut self, x: i32, z: i32) {
        self.columns.remove(&(x, z));
    }

    /// Forgets every mesh (leaving a world or the title screen).
    pub fn clear(&mut self) {
        self.columns.clear();
        self.outline = None;
    }

    /// Back to front, so that blending sees what is behind each surface.
    fn draw_translucent(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        camera: &Camera,
        frustum: &Frustum,
    ) {
        let mut translucent: Vec<(f64, &GpuSection)> = self
            .visible_sections(camera, frustum)
            .filter(|s| s.translucent.is_some())
            .map(|s| {
                (
                    s.origin.distance_squared(camera.position - SECTION_CENTER),
                    s,
                )
            })
            .collect();
        if translucent.is_empty() {
            return;
        }
        translucent.sort_by(|a, b| b.0.total_cmp(&a.0));
        self.blocks.bind(pass, BlockPass::Translucent);
        for (_, section) in translucent {
            if let Some(mesh) = &section.translucent {
                BlockPipeline::set_section_offset(pass, camera.relative(section.origin));
                mesh.draw(pass);
            }
        }
    }
}

const SECTION_CENTER: DVec3 = DVec3::splat(SECTION_SIZE as f64 / 2.0);

fn texture_array(
    gpu: &Gpu,
    textures: &TextureLibrary,
    filtering: TextureFiltering,
) -> Result<TextureArray, GpuError> {
    let layers: Vec<_> = textures.images.iter().map(|(_, _, image)| image).collect();
    TextureArray::new(&gpu.device, &gpu.queue, &layers, filtering)
}
