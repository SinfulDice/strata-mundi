use super::Gpu;
use std::sync::Arc;
use winit::window::Window;

/// A window and its swapchain. Several can share one `Gpu` (multi-screen ready).
pub struct WindowSurface {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    /// Reconfiguring while a frame is alive is invalid: it waits for the next acquire.
    needs_reconfigure: bool,
}

pub enum Acquired {
    Frame(wgpu::SurfaceTexture),
    /// Nothing to draw this time (minimized, timeout, surface just reconfigured).
    Skip,
}

impl WindowSurface {
    pub fn new(
        gpu: &Gpu,
        window: Arc<Window>,
        surface: wgpu::Surface<'static>,
        vsync: bool,
    ) -> Self {
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
            .unwrap_or_else(|| fallback_config(size.width, size.height));
        let capabilities = surface.get_capabilities(&gpu.adapter);
        if let Some(srgb) = capabilities.formats.iter().find(|f| f.is_srgb()) {
            config.format = *srgb;
        }
        config.present_mode = present_mode(&capabilities.present_modes, vsync);
        config.desired_maximum_frame_latency = 2;
        surface.configure(&gpu.device, &config);
        Self {
            window,
            surface,
            config,
            needs_reconfigure: false,
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn present_mode(&self) -> wgpu::PresentMode {
        self.config.present_mode
    }

    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    pub fn resize(&mut self, gpu: &Gpu, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&gpu.device, &self.config);
    }

    pub fn acquire(&mut self, gpu: &Gpu) -> Acquired {
        if std::mem::take(&mut self.needs_reconfigure) {
            self.surface.configure(&gpu.device, &self.config);
        }
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => Acquired::Frame(frame),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.needs_reconfigure = true;
                Acquired::Frame(frame)
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                tracing::debug!("surface outdated or lost, reconfiguring");
                self.surface.configure(&gpu.device, &self.config);
                Acquired::Skip
            }
            other => {
                tracing::trace!(status = ?std::mem::discriminant(&other), "frame skipped");
                Acquired::Skip
            }
        }
    }
}

fn present_mode(available: &[wgpu::PresentMode], vsync: bool) -> wgpu::PresentMode {
    let preferred: &[wgpu::PresentMode] = if vsync {
        &[wgpu::PresentMode::Fifo]
    } else {
        &[wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate]
    };
    preferred
        .iter()
        .copied()
        .find(|mode| available.contains(mode))
        .unwrap_or(wgpu::PresentMode::Fifo)
}

fn fallback_config(width: u32, height: u32) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        color_space: Default::default(),
    }
}
