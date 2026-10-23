use super::Gpu;
use std::sync::mpsc;
use strata_core::texture::RgbaImage;
use thiserror::Error;

pub const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[derive(Debug, Error)]
pub enum ReadbackError {
    #[error("the GPU did not finish rendering: {0}")]
    Poll(#[from] wgpu::PollError),
    #[error("could not read the rendered image back: {0}")]
    Map(String),
}

/// A color texture that can be rendered to and read back to the CPU.
pub struct OffscreenTarget {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: (u32, u32),
}

impl OffscreenTarget {
    pub fn new(gpu: &Gpu, size: (u32, u32)) -> Self {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen target"),
            size: extent(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&Default::default()),
            texture,
            size,
        }
    }

    /// Copies the texture into `encoder`, submits it and blocks until the pixels are on the CPU.
    pub fn read(
        &self,
        gpu: &Gpu,
        mut encoder: wgpu::CommandEncoder,
    ) -> Result<RgbaImage, ReadbackError> {
        let (width, height) = self.size;
        let padded_row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            extent(self.size),
        );
        gpu.queue.submit([encoder.finish()]);
        let (sender, receiver) = mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result);
        });
        gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver
            .recv()
            .map_err(|e| ReadbackError::Map(e.to_string()))?
            .map_err(|e| ReadbackError::Map(e.to_string()))?;
        let mapped = buffer
            .get_mapped_range(..)
            .map_err(|e| ReadbackError::Map(e.to_string()))?;
        unpad(&mapped, width, height, padded_row)
            .ok_or_else(|| ReadbackError::Map("unexpected image size".into()))
    }
}

fn unpad(bytes: &[u8], width: u32, height: u32, padded_row: u32) -> Option<RgbaImage> {
    let row = (width * 4) as usize;
    let pixels = bytes
        .chunks(padded_row as usize)
        .take(height as usize)
        .flat_map(|padded| &padded[..row])
        .copied()
        .collect();
    RgbaImage::from_bytes(width, height, pixels)
}

fn extent((width, height): (u32, u32)) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}
