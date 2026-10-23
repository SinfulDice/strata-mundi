use super::GpuError;
use strata_core::texture::{Rgba, RgbaImage};

/// How block textures are sampled (graphics settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureFiltering {
    /// Smaller copies for distant blocks: less shimmering, a bit more memory.
    pub mipmaps: bool,
    /// 1 = off. Above 1, sharper textures at grazing angles but smoothed pixels up close
    /// (the GPU requires linear filtering for it).
    pub anisotropy: u16,
}

impl Default for TextureFiltering {
    fn default() -> Self {
        Self {
            mipmaps: false,
            anisotropy: 1,
        }
    }
}

/// All block textures in one GPU texture array: one layer per texture.
pub struct TextureArray {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl TextureArray {
    /// Layers all have the library resolution (`TextureLibrary` scales them on load).
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layers: &[&RgbaImage],
        filtering: TextureFiltering,
    ) -> Result<Self, GpuError> {
        let max = device.limits().max_texture_array_layers;
        if layers.len() > max as usize {
            return Err(GpuError::TooManyTextures {
                count: layers.len(),
                max,
            });
        }
        let (width, height) = layers.first().map_or((1, 1), |l| (l.width(), l.height()));
        let levels = if filtering.mipmaps {
            width.min(height).max(1).ilog2() + 1
        } else {
            1
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("block textures"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers.len().max(1) as u32,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (index, layer) in layers.iter().enumerate() {
            let mut image = (*layer).clone();
            for level in 0..levels {
                if level > 0 {
                    image = half(&image);
                }
                upload(queue, &texture, &image, level, index as u32);
            }
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&sampler_for(filtering));
        Ok(Self { view, sampler })
    }
}

fn sampler_for(filtering: TextureFiltering) -> wgpu::SamplerDescriptor<'static> {
    let anisotropic = filtering.anisotropy > 1;
    let pixel = if anisotropic {
        wgpu::FilterMode::Linear
    } else {
        wgpu::FilterMode::Nearest
    };
    wgpu::SamplerDescriptor {
        label: Some("block sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: pixel,
        min_filter: pixel,
        mipmap_filter: if filtering.mipmaps || anisotropic {
            wgpu::MipmapFilterMode::Linear
        } else {
            wgpu::MipmapFilterMode::Nearest
        },
        anisotropy_clamp: filtering.anisotropy.clamp(1, 16),
        ..Default::default()
    }
}

fn upload(queue: &wgpu::Queue, texture: &wgpu::Texture, image: &RgbaImage, level: u32, layer: u32) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: level,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: layer,
            },
            aspect: wgpu::TextureAspect::All,
        },
        image.as_bytes(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * image.width()),
            rows_per_image: Some(image.height()),
        },
        wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
    );
}

/// Half-size copy; colours weighted by alpha so transparent pixels do not darken edges.
fn half(image: &RgbaImage) -> RgbaImage {
    let (w, h) = ((image.width() / 2).max(1), (image.height() / 2).max(1));
    let mut out = RgbaImage::new(w, h, Rgba([0; 4]));
    for y in 0..h {
        for x in 0..w {
            let pixels = [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| {
                let (sx, sy) = (
                    (x * 2 + dx).min(image.width() - 1),
                    (y * 2 + dy).min(image.height() - 1),
                );
                image.get(sx, sy).0.map(u32::from)
            });
            let alpha: u32 = pixels.iter().map(|p| p[3]).sum();
            let channel = |i: usize| {
                let weighted: u32 = pixels.iter().map(|p| p[i] * p[3]).sum();
                (weighted / alpha.max(1)) as u8
            };
            out.set(
                x,
                y,
                Rgba([channel(0), channel(1), channel(2), (alpha / 4) as u8]),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halving_ignores_transparent_colours() {
        let mut image = RgbaImage::new(2, 2, Rgba([0, 0, 0, 0]));
        image.set(0, 0, Rgba([200, 100, 50, 255]));
        let small = half(&image);
        assert_eq!(small.get(0, 0), Rgba([200, 100, 50, 63]));
    }

    #[test]
    fn anisotropy_switches_to_linear_filtering() {
        let off = sampler_for(TextureFiltering::default());
        assert_eq!(off.mag_filter, wgpu::FilterMode::Nearest);
        let on = sampler_for(TextureFiltering {
            mipmaps: true,
            anisotropy: 8,
        });
        assert_eq!(
            (on.mag_filter, on.anisotropy_clamp),
            (wgpu::FilterMode::Linear, 8)
        );
    }
}
