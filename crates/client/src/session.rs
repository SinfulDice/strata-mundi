use crate::app::{ClientConfig, ClientError};
use crate::bench::BenchRun;
use crate::camera::Camera;
use crate::fly::FlyController;
use crate::input::{Action, Binding, Bindings, Gamepads, InputState};
use crate::render::{Acquired, Gpu, Renderer, Target, WindowSurface};
use glam::Vec2;
use std::sync::Arc;
use std::time::Instant;
use strata_protocol::{ClientConnection, Disconnected, ServerMessage};
use winit::event::{DeviceEvent, ElementState, WindowEvent};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Window};

const MAX_FRAME_SECONDS: f32 = 0.1;

/// Everything that lives while the game window is open.
pub struct Session {
    pub gpu: Gpu,
    pub surface: WindowSurface,
    pub connection: ClientConnection,
    pub bench: Option<BenchRun>,
    renderer: Renderer,
    camera: Camera,
    fly: FlyController,
    input: InputState,
    gamepads: Gamepads,
    cursor_captured: bool,
    focused: bool,
    last_frame: Instant,
    server_tick: u64,
}

impl Session {
    pub fn new(
        window: Arc<Window>,
        config: &ClientConfig,
        connection: ClientConnection,
    ) -> Result<Self, ClientError> {
        let instance = Gpu::create_instance();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(crate::render::GpuError::from)?;
        let gpu = Gpu::new(instance, Some(&surface))?;
        let surface = WindowSurface::new(&gpu, window, surface, config.vsync);
        let mut renderer = Renderer::new(&gpu, surface.format(), &config.content.textures);
        renderer.add_object(&gpu, &config.scene, glam::DVec3::ZERO);
        Ok(Self {
            gpu,
            surface,
            connection,
            bench: None,
            renderer,
            camera: config.camera.clone(),
            fly: FlyController::default(),
            input: InputState::new(Bindings::default()),
            gamepads: Gamepads::connect(),
            cursor_captured: false,
            focused: true,
            last_frame: Instant::now(),
            server_tick: 0,
        })
    }

    pub fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => self.surface.resize(&self.gpu, size.width, size.height),
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                if !focused {
                    self.set_cursor_captured(false);
                    self.input.release_all();
                }
            }
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.input
                        .set_binding(Binding::Key(code), event.state.is_pressed());
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if !self.cursor_captured && state == ElementState::Pressed {
                    self.set_cursor_captured(true);
                    return;
                }
                self.input
                    .set_binding(Binding::Mouse(button), state.is_pressed());
            }
            WindowEvent::Touch(_) => self.input.note_touch(),
            _ => {}
        }
    }

    pub fn handle_device_event(&mut self, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event
            && self.cursor_captured
        {
            self.input
                .add_mouse_motion(Vec2::new(delta.0 as f32, delta.1 as f32));
        }
    }

    /// Updates and renders one frame. Returns `false` when the client should quit.
    pub fn draw(&mut self) -> Result<bool, ClientError> {
        let start = Instant::now();
        let frame_seconds = (start - self.last_frame).as_secs_f32();
        self.last_frame = start;
        self.update(frame_seconds)?;
        let cpu_seconds = self.render();
        if let Some(bench) = &mut self.bench {
            if let Some(cpu_seconds) = cpu_seconds {
                bench.record_frame(
                    &self.gpu,
                    frame_seconds * 1000.0,
                    cpu_seconds * 1000.0,
                    self.server_tick,
                );
            }
            if bench.is_finished() {
                let path = bench
                    .write_report(&self.gpu, self.surface.size(), self.surface.present_mode())
                    .map_err(ClientError::Report)?;
                tracing::info!(path = %path.display(), "performance report written");
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The bench follows real time; interactive play caps `dt` so a stall cannot teleport.
    fn update(&mut self, frame_seconds: f32) -> Result<(), ClientError> {
        if self.focused {
            self.gamepads.poll(&mut self.input);
        }
        self.receive_messages()?;
        if self.input.just_pressed(Action::Pause) {
            self.set_cursor_captured(false);
        }
        match &mut self.bench {
            Some(bench) => bench.advance(frame_seconds, &mut self.camera),
            None => self.fly.update(
                &mut self.camera,
                &self.input,
                frame_seconds.min(MAX_FRAME_SECONDS),
            ),
        }
        self.input.end_frame();
        Ok(())
    }

    fn receive_messages(&mut self) -> Result<(), ClientError> {
        loop {
            match self.connection.try_recv() {
                Ok(Some(ServerMessage::Tick { tick })) => self.server_tick = tick,
                Ok(None) => return Ok(()),
                Err(Disconnected) => return Err(ClientError::ServerGone),
            }
        }
    }

    /// Returns the CPU time spent building and submitting the frame, in seconds,
    /// or `None` when no frame could be drawn.
    fn render(&mut self) -> Option<f32> {
        let start = Instant::now();
        let Acquired::Frame(frame) = self.surface.acquire(&self.gpu) else {
            return None;
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let timestamps = self.bench.as_mut().and_then(BenchRun::timestamp_writes);
        self.renderer.render(
            &self.gpu,
            &mut encoder,
            Target {
                view: &view,
                size: self.surface.size(),
            },
            &self.camera,
            timestamps,
        );
        if let Some(bench) = &self.bench {
            bench.resolve(&mut encoder);
        }
        self.gpu.queue.submit([encoder.finish()]);
        if let Some(bench) = &mut self.bench {
            bench.after_submit();
        }
        let cpu = start.elapsed().as_secs_f32();
        self.surface.window.pre_present_notify();
        self.gpu.queue.present(frame);
        Some(cpu)
    }

    fn set_cursor_captured(&mut self, captured: bool) {
        if self.cursor_captured == captured {
            return;
        }
        let window = &self.surface.window;
        let grab = if captured {
            window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        } else {
            window.set_cursor_grab(CursorGrabMode::None)
        };
        if let Err(error) = grab {
            tracing::warn!(%error, "cursor grab unavailable");
        }
        window.set_cursor_visible(!captured);
        self.cursor_captured = captured;
    }
}
