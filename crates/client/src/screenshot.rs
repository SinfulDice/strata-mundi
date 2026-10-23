use crate::camera::Camera;
use crate::mesher::{BlockTable, mesh_section};
use crate::render::{
    Gpu, GpuError, OFFSCREEN_FORMAT, OffscreenTarget, ReadbackError, Renderer, Target,
    TextureFiltering,
};
use crate::settings::GraphicsSettings;
use crate::world::ClientWorld;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use strata_core::content::Content;
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
}

/// Renders one frame without any window (works headless with Mesa lavapipe) and saves it as PNG.
pub fn capture(
    content: &Content,
    world: &ClientWorld,
    view: &View,
    path: &Path,
) -> Result<(), ScreenshotError> {
    let image = render_image(content, world, view)?;
    write_png(path, &image)?;
    tracing::info!(path = %path.display(), "screenshot saved");
    Ok(())
}

pub fn render_image(
    content: &Content,
    world: &ClientWorld,
    view: &View,
) -> Result<RgbaImage, ScreenshotError> {
    let gpu = Gpu::new(Gpu::create_instance(), None)?;
    let filtering = TextureFiltering {
        mipmaps: view.graphics.mipmaps,
        anisotropy: view.graphics.anisotropy.clamp(1, 16) as u16,
    };
    let mut renderer = Renderer::new(&gpu, OFFSCREEN_FORMAT, &content.textures, filtering)?;
    let ao = view.graphics.ambient_occlusion;
    renderer.ambient_occlusion = f32::from(u8::from(ao));
    renderer.vertical_distance = view.graphics.vertical_distance;
    let table = BlockTable::new(content.blocks.iter().map(|(_, _, block)| block));
    for key in world.section_keys() {
        let mesh = mesh_section(&world.neighborhood(key), &table, ao);
        renderer.upload_section(&gpu, key, &mesh);
    }
    let size = view.size;
    let camera = &view.camera;
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
