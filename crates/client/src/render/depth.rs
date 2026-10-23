pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct DepthBuffer {
    pub view: wgpu::TextureView,
    size: (u32, u32),
}

impl DepthBuffer {
    pub fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&Default::default()),
            size,
        }
    }

    /// Recreates the buffer only when the target size changed.
    pub fn ensure_size(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.size != size {
            *self = Self::new(device, size);
        }
    }
}
