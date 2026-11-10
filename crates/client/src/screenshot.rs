use crate::camera::Camera;
use crate::mesher::{BlockTable, MeshOptions, mesh_section};
use crate::models::ModelLibrary;
use crate::particles::{ParticleSystem, ParticleTable};
use crate::render::{
    Gpu, GpuError, OFFSCREEN_FORMAT, OffscreenTarget, ReadbackError, RenderOptions, Renderer,
    Target, TextureFiltering,
};
use crate::settings::{GraphicsSettings, Lighting, ParticleMode};
use crate::sky::SkyFrame;
use crate::world::ClientWorld;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::dimension::DimensionId;
use strata_core::light::LightTable;
use strata_core::sky::Moment;
use strata_core::texture::RgbaImage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScreenshotError {
    #[error(transparent)]
    Gpu(#[from] GpuError),
    #[error(transparent)]
    Readback(#[from] ReadbackError),
    #[error("could not write {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not encode {path}: {source}")]
    Png {
        path: PathBuf,
        source: png::EncodingError,
    },
}

/// What to render and how.
pub struct View {
    pub camera: Camera,
    pub size: (u32, u32),
    pub graphics: GraphicsSettings,
    pub sky: SkyFrame,
    pub particles: ParticleShot,
}

/// Particles frozen at one moment: a block broken at the start, then `seconds` of
/// simulation (drips need a few to appear).
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleShot {
    pub broken: Option<BlockPos>,
    pub seconds: f32,
}

/// Simulation step of the particles of a capture.
const PARTICLE_STEP: f32 = 1.0 / 60.0;

impl ParticleShot {
    /// Breaks the block (it leaves the world before light and meshing) and returns the
    /// particles thrown.
    fn start(
        &self,
        content: &Content,
        world: &mut ClientWorld,
        mode: ParticleMode,
    ) -> ParticleSystem {
        let mut particles = ParticleSystem::new(ParticleTable::from_content(content), mode);
        if let Some(pos) = self.broken {
            particles.block_changed(pos, world.block_at(pos), BlockId::AIR, world);
            world.set_block(pos.x, pos.y, pos.z, BlockId::AIR);
        }
        particles
    }

    fn run(&self, particles: &mut ParticleSystem, world: &ClientWorld, camera: &Camera) {
        for _ in 0..(self.seconds / PARTICLE_STEP).round() as u32 {
            particles.update(PARTICLE_STEP, camera.position, world);
        }
    }
}

/// The sky of `dimension` at world time `time` (ticks; noon if `None`).
pub fn capture_sky(
    content: &Content,
    dimension: &DimensionId,
    time: Option<u64>,
    graphics: &GraphicsSettings,
) -> SkyFrame {
    let cycle = content.day_cycle(dimension);
    let time = time.unwrap_or_else(|| cycle.moment(Moment::Noon));
    SkyFrame::at(&content.sky(dimension), &cycle, time as f64, graphics.stars)
}

/// Renders one frame without any window (works headless with Mesa lavapipe) and saves it as PNG.
pub fn capture(
    content: &Content,
    world: &mut ClientWorld,
    view: &View,
    path: &Path,
) -> Result<(), ScreenshotError> {
    let image = render_image(content, world, view)?;
    write_png(path, &image)?;
    tracing::info!(path = %path.display(), "screenshot saved");
    Ok(())
}

/// The world is lit first, so its light is complete before meshing.
pub fn render_image(
    content: &Content,
    world: &mut ClientWorld,
    view: &View,
) -> Result<RgbaImage, ScreenshotError> {
    let mut particles = view
        .particles
        .start(content, world, view.graphics.particles);
    match view.graphics.lighting {
        Lighting::Full => world.compute_light(&LightTable::from_content(content)),
        Lighting::Flat => world.light_flat(),
    }
    let gpu = Gpu::new(Gpu::create_instance(), None)?;
    let filtering = TextureFiltering {
        mipmaps: view.graphics.mipmaps,
        anisotropy: view.graphics.anisotropy.clamp(1, 16) as u16,
    };
    let mut renderer = Renderer::new(&gpu, OFFSCREEN_FORMAT, &content.textures, filtering)?;
    let ao = view.graphics.ambient_occlusion;
    renderer.ambient_occlusion = f32::from(u8::from(ao));
    renderer.vertical_distance = view.graphics.vertical_distance;
    renderer.options = RenderOptions::for_capture(&view.graphics);
    let position = view.camera.position;
    let fog_tint = world.fog_tint_at(position.x, position.z);
    renderer.sky = view.sky.with_fog_tint(fog_tint.unwrap_or(glam::Vec3::ONE));
    renderer.models.library = ModelLibrary::from_content(content);
    renderer.models.entities = world.entity_models().collect();
    let table = BlockTable::from_content(content);
    if world.is_inside_opaque(view.camera.position, &table) {
        tracing::warn!(
            position = ?view.camera.position,
            "camera inside a solid block: the surrounding rock is invisible from inside"
        );
    }
    for key in world.section_keys() {
        let mesh = mesh_section(
            &world.neighborhood(key, &table, view.graphics.biome_blend),
            &table,
            MeshOptions::from_graphics(&view.graphics),
        );
        renderer.upload_section(&gpu, key, &mesh);
    }
    let size = view.size;
    let camera = &view.camera;
    view.particles.run(&mut particles, world, camera);
    particles.write_instances(camera.position, world, &mut renderer.particles.instances);
    renderer.immersion = world.immersion(camera.position, content);
    let target = OffscreenTarget::new(&gpu, size);
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    renderer.render(
        &gpu,
        &mut encoder,
        Target {
            view: &target.view,
            size: target.size,
        },
        camera,
        None,
    );
    Ok(target.read(&gpu, encoder)?)
}

/// PNG file contents (world captures sent to the server).
pub fn encode_png(image: &RgbaImage) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, image.width(), image.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(image.as_bytes()).ok()?;
    writer.finish().ok()?;
    Some(bytes)
}

pub fn write_png(path: &Path, image: &RgbaImage) -> Result<(), ScreenshotError> {
    let io_error = |source| ScreenshotError::Io {
        path: path.to_owned(),
        source,
    };
    let png_error = |source| ScreenshotError::Png {
        path: path.to_owned(),
        source,
    };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(io_error)?;
    }
    let file = File::create(path).map_err(io_error)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), image.width(), image.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(png_error)?;
    writer
        .write_image_data(image.as_bytes())
        .map_err(png_error)?;
    writer.finish().map_err(png_error)
}
