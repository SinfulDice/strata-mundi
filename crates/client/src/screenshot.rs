use crate::camera::Camera;
use crate::render::{
    Gpu, GpuError, MeshData, OFFSCREEN_FORMAT, OffscreenTarget, ReadbackError, Renderer, Target,
};
use glam::DVec3;
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

pub struct ScreenshotRequest {
    pub path: PathBuf,
    pub size: (u32, u32),
    pub camera: Camera,
}

/// Renders one frame without any window (works headless with Mesa lavapipe) and saves it as PNG.
pub fn capture(
    content: &Content,
    scene: &MeshData,
    request: &ScreenshotRequest,
) -> Result<(), ScreenshotError> {
    let image = render_image(content, scene, &request.camera, request.size)?;
    write_png(&request.path, &image)?;
    tracing::info!(path = %request.path.display(), "screenshot saved");
    Ok(())
}

pub fn render_image(
    content: &Content,
    scene: &MeshData,
    camera: &Camera,
    size: (u32, u32),
) -> Result<RgbaImage, ScreenshotError> {
    let gpu = Gpu::new(Gpu::create_instance(), None)?;
    let mut renderer = Renderer::new(&gpu, OFFSCREEN_FORMAT, &content.textures);
    renderer.add_object(&gpu, scene, DVec3::ZERO);
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
