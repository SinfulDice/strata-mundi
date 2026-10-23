use super::block_pipeline::MESH_OFFSET_SIZE;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("no Vulkan graphics adapter found: {0}")]
    NoAdapter(#[from] wgpu::RequestAdapterError),
    #[error("could not open the graphics device: {0}")]
    NoDevice(#[from] wgpu::RequestDeviceError),
    #[error("could not create the window surface: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
}

/// The graphics device shared by every window and off-screen target.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// Vulkan only, on every platform: a single rendering path to test.
    pub fn create_instance() -> wgpu::Instance {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        descriptor.backends = wgpu::Backends::VULKAN;
        wgpu::Instance::new(descriptor)
    }

    pub fn new(
        instance: wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, GpuError> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface,
            apply_limit_buckets: false,
        }))?;
        let info = adapter.get_info();
        tracing::info!(name = %info.name, driver = %info.driver, kind = ?info.device_type, "graphics adapter");
        let optional_features = wgpu::Features::TIMESTAMP_QUERY;
        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("strata device"),
                required_features: wgpu::Features::IMMEDIATES
                    | (adapter.features() & optional_features),
                required_limits: wgpu::Limits {
                    max_immediate_size: MESH_OFFSET_SIZE,
                    ..Default::default()
                }
                .using_resolution(adapter.limits()),
                ..Default::default()
            }),
        )?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub fn supports_timestamps(&self) -> bool {
        self.device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
    }
}
