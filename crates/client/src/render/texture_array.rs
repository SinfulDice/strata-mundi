use strata_core::texture::RgbaImage;

/// All block textures in one GPU texture array: one layer per texture, nearest filtering.
pub struct TextureArray {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl TextureArray {
    /// Layers all have the library resolution (`TextureLibrary` scales them on load).
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layers: &[&RgbaImage]) -> Self {
        let (width, height) = layers.first().map_or((1, 1), |l| (l.width(), l.height()));
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layers.len().max(1) as u32,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("block textures"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (index, layer) in layers.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: index as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                layer.as_bytes(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("block sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Self { view, sampler }
    }
}
