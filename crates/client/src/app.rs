use crate::bench::{BenchConfig, BenchRun};
use crate::camera::Camera;
use crate::render::GpuError;
use crate::session::Session;
use std::sync::Arc;
use strata_protocol::{ClientConnection, ClientMessage};
use thiserror::Error;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
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
}

pub struct ClientConfig {
    pub camera: Camera,
    pub vsync: bool,
    pub bench: Option<BenchConfig>,
}

/// Opens the game window and runs until it is closed.
pub fn run(config: ClientConfig, connection: ClientConnection) -> Result<(), ClientError> {
    let event_loop = EventLoop::new()?;
    let mut app = ClientApp {
        pending: Some((config, connection)),
        session: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;
    if let Some(session) = app.session.take() {
        session.connection.send(ClientMessage::Disconnect);
    }
    app.error.map_or(Ok(()), Err)
}

struct ClientApp {
    pending: Option<(ClientConfig, ClientConnection)>,
    session: Option<Session>,
    error: Option<ClientError>,
}

impl ClientApp {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: ClientError) {
        tracing::error!(%error, "client stopped");
        self.error = Some(error);
        event_loop.exit();
    }

    fn start(
        event_loop: &ActiveEventLoop,
        config: ClientConfig,
        connection: ClientConnection,
    ) -> Result<Session, ClientError> {
        let attributes = Window::default_attributes()
            .with_title("Strata Mundi")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let mut session = Session::new(window, config.camera, config.vsync, connection)?;
        if let Some(bench) = config.bench {
            session.bench = Some(BenchRun::new(bench, &session.gpu));
        }
        Ok(session)
    }
}

impl ApplicationHandler for ClientApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some((config, connection)) = self.pending.take() else {
            return;
        };
        match Self::start(event_loop, config, connection) {
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

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(session) = &self.session {
            session.surface.window.request_redraw();
        }
    }
}
