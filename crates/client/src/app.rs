use crate::bench::BenchConfig;
use crate::launcher::{Launcher, RunningWorld};
use crate::render::GpuError;
use crate::session::Session;
use crate::settings::Settings;
use crate::ui::UiAssets;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use strata_core::content::Content;
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("could not open a window: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("could not create the window: {0}")]
    Window(#[from] winit::error::OsError),
    #[error(transparent)]
    Gpu(#[from] GpuError),
    #[error("could not write the performance report: {0}")]
    Report(std::io::Error),
    #[error("the server stopped unexpectedly")]
    ServerGone,
    #[error("could not start worker threads: {0}")]
    Threads(std::io::Error),
}

/// Vertical field of view, in degrees, of the tools without settings (captures).
pub const DEFAULT_FOV: f32 = 70.0;

pub struct ClientConfig {
    pub content: Arc<Content>,
    pub ui: UiAssets,
    pub settings: Settings,
    /// Where setting changes are written.
    pub settings_path: PathBuf,
    pub bench: Option<BenchConfig>,
    pub launcher: Box<dyn Launcher>,
    /// Start in this world directly (benchmarks, `--seed`) instead of the title screen.
    pub start: Option<RunningWorld>,
}

/// Opens the game window and runs until it is closed; the world being played is saved.
pub fn run(config: ClientConfig) -> Result<(), ClientError> {
    let event_loop = EventLoop::new()?;
    let mut app = ClientApp {
        pending: Some(config),
        session: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;
    if let Some(mut session) = app.session.take() {
        session.leave_world();
    }
    app.error.map_or(Ok(()), Err)
}

struct ClientApp {
    pending: Option<ClientConfig>,
    session: Option<Session>,
    error: Option<ClientError>,
}

impl ClientApp {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: ClientError) {
        tracing::error!(%error, "client stopped");
        self.error = Some(error);
        event_loop.exit();
    }

    fn start(event_loop: &ActiveEventLoop, config: ClientConfig) -> Result<Session, ClientError> {
        let title = config
            .content
            .translations
            .text(&config.settings.interface.language, "game-title");
        let attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
        let window = Arc::new(event_loop.create_window(attributes)?);
        Session::new(window, config)
    }
}

impl ApplicationHandler for ClientApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(config) = self.pending.take() else {
            return;
        };
        match Self::start(event_loop, config) {
            Ok(session) => self.session = Some(session),
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(session) = &mut self.session else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => match session.draw() {
                Ok(true) => {}
                Ok(false) => event_loop.exit(),
                Err(error) => self.fail(event_loop, error),
            },
            other => session.handle_window_event(other),
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let Some(session) = &mut self.session {
            session.handle_device_event(event);
        }
    }

    /// Draws at once, or sleeps until the next frame allowed by the FPS limit (or the
    /// slow pace of a window in the background).
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(session) = &self.session else {
            return;
        };
        match session.next_frame_at() {
            Some(at) if at > Instant::now() => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(at));
            }
            _ => {
                event_loop.set_control_flow(ControlFlow::Poll);
                session.surface.window.request_redraw();
            }
        }
    }
}
