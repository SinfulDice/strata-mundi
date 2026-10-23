use crate::app::{ClientConfig, ClientError, DEFAULT_FOV};
use crate::bench::BenchRun;
use crate::camera::Camera;
use crate::gameplay::Gameplay;
use crate::input::{Action, Binding, Bindings, Gamepads, InputState};
use crate::interaction::BlockInfo;
use crate::mesh_scheduler::MeshScheduler;
use crate::mesher::BlockTable;
use crate::render::{Acquired, Gpu, Renderer, Target, WindowSurface};
use crate::screens::Screens;
use crate::settings::GraphicsSettings;
use crate::streaming::WorldView;
use crate::world::ClientWorld;
use glam::{DVec3, Vec2};
use std::sync::Arc;
use std::time::Instant;
use strata_protocol::{
    ClientConnection, ClientMessage, Disconnected, MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE,
    ServerMessage,
};
use winit::event::{DeviceEvent, ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Window};

const MAX_FRAME_SECONDS: f32 = 0.1;
/// Pixels of trackpad scrolling counted as one wheel notch.
const PIXELS_PER_NOTCH: f64 = 40.0;

/// Everything that lives while the game window is open.
pub struct Session {
    pub gpu: Gpu,
    pub surface: WindowSurface,
    pub connection: ClientConnection,
    pub bench: Option<BenchRun>,
    renderer: Renderer,
    screens: Screens,
    game: Gameplay,
    graphics: GraphicsSettings,
    input: InputState,
    idle: InputState,
    gamepads: Gamepads,
    cursor_captured: bool,
    focused: bool,
    last_frame: Instant,
    server_tick: u64,
}

impl Session {
    pub fn new(
        window: Arc<Window>,
        config: ClientConfig,
        connection: ClientConnection,
    ) -> Result<Self, ClientError> {
        let instance = Gpu::create_instance();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(crate::render::GpuError::from)?;
        let gpu = Gpu::new(instance, Some(&surface))?;
        let surface = WindowSurface::new(&gpu, window, surface, config.vsync);
        let mut renderer = Renderer::new(&gpu, surface.format(), &config.content.textures)?;
        let graphics = config.graphics;
        renderer.ambient_occlusion = f32::from(u8::from(graphics.ambient_occlusion));
        renderer.vertical_distance = graphics.vertical_distance;
        let table = Arc::new(BlockTable::new(
            config.content.blocks.iter().map(|(_, _, b)| b),
        ));
        let meshing =
            MeshScheduler::new(table, graphics.worker_threads, graphics.ambient_occlusion)
                .map_err(ClientError::Threads)?;
        let bench = config.bench.map(|bench| BenchRun::new(bench, &gpu));
        let render_distance = graphics
            .render_distance
            .clamp(MIN_VIEW_DISTANCE, MAX_VIEW_DISTANCE);
        connection.send(ClientMessage::ViewDistance(render_distance));
        let camera = Camera::new(DVec3::ZERO, 0.0, 0.0, DEFAULT_FOV);
        let screens = Screens::new(
            &gpu,
            surface.format(),
            config.ui,
            &config.content,
            config.language,
        );
        let game = Gameplay::new(
            WorldView::new(ClientWorld::default(), meshing),
            camera,
            BlockInfo::new(config.content),
            config.controls,
        );
        Ok(Self {
            gpu,
            surface,
            connection,
            bench,
            renderer,
            screens,
            game,
            graphics,
            input: InputState::new(Bindings::default()),
            idle: InputState::new(Bindings::default()),
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
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(text) = &event.text
                    && event.state.is_pressed()
                {
                    self.input.add_text(text);
                }
                if let (PhysicalKey::Code(code), false) = (event.physical_key, event.repeat) {
                    self.input
                        .set_binding(Binding::Key(code), event.state.is_pressed());
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if !self.cursor_captured
                    && !self.screens.is_menu_open()
                    && state == ElementState::Pressed
                {
                    self.set_cursor_captured(true);
                    return;
                }
                self.input
                    .set_binding(Binding::Mouse(button), state.is_pressed());
            }
            WindowEvent::MouseWheel { delta, .. } => self.input.add_scroll(match delta {
                MouseScrollDelta::LineDelta(_, y) => y,
                MouseScrollDelta::PixelDelta(p) => (p.y / PIXELS_PER_NOTCH) as f32,
            }),
            WindowEvent::CursorMoved { position, .. } => {
                self.input.set_cursor((position.x, position.y))
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
        let was_open = self.screens.is_menu_open();
        let used_by_menu = self.screens.handle(&self.input, &mut self.game);
        if self.screens.is_menu_open() {
            self.set_cursor_captured(false);
        } else if was_open {
            self.set_cursor_captured(true);
        }
        if !used_by_menu {
            self.handle_system_actions();
        }
        let settled = self
            .game
            .view
            .is_settled(settled_columns(self.graphics.render_distance));
        match &mut self.bench {
            // The bench starts once the initial world is loaded, so loading is not measured.
            Some(bench) if settled || bench.started() => {
                bench.advance(frame_seconds, &mut self.game.camera);
                self.game.follow_camera(&self.connection);
            }
            Some(_) => {}
            // Without the cursor (paused, menus), the world goes on but the player is idle.
            None => {
                let playing = self.cursor_captured && !self.screens.is_menu_open();
                let input = if playing { &self.input } else { &self.idle };
                self.game.update(
                    frame_seconds.min(MAX_FRAME_SECONDS),
                    input,
                    &self.connection,
                );
            }
        }
        self.input.end_frame();
        self.renderer.outline = self.game.interaction.target.map(|hit| hit.block);
        let camera = self.game.camera.position;
        self.game.view.update(
            &self.gpu,
            &mut self.renderer,
            camera,
            self.graphics.vertical_distance,
        );
        Ok(())
    }

    fn handle_system_actions(&mut self) {
        if self.input.just_pressed(Action::Pause) {
            self.set_cursor_captured(false);
        }
        if self.input.just_pressed(Action::RenderDistanceDown) {
            self.set_render_distance(self.graphics.render_distance.saturating_sub(1));
        }
        if self.input.just_pressed(Action::RenderDistanceUp) {
            self.set_render_distance(self.graphics.render_distance + 1);
        }
    }

    fn receive_messages(&mut self) -> Result<(), ClientError> {
        loop {
            match self.connection.try_recv() {
                Ok(Some(message)) => self.apply(message),
                Ok(None) => return Ok(()),
                Err(Disconnected) => return Err(ClientError::ServerGone),
            }
        }
    }

    fn apply(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Tick { tick } => self.server_tick = tick,
            ServerMessage::Welcome { body, .. } if self.bench.is_some() => {
                if let Some(bench) = &mut self.bench {
                    bench.set_origin(body.position);
                }
                self.game
                    .apply(message, &mut self.renderer, &self.connection);
            }
            other => self.game.apply(other, &mut self.renderer, &self.connection),
        }
    }

    /// Changes how far the world is loaded, while playing.
    pub fn set_render_distance(&mut self, distance: u32) {
        let distance = distance.clamp(MIN_VIEW_DISTANCE, MAX_VIEW_DISTANCE);
        tracing::info!(distance, "render distance");
        self.graphics.render_distance = distance;
        self.connection.send(ClientMessage::ViewDistance(distance));
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
            &self.game.camera,
            timestamps,
        );
        self.screens.draw(
            &self.gpu,
            &mut encoder,
            &view,
            self.surface.size(),
            &mut self.game,
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

/// About 80 % of the columns inside the view circle: enough to call the world loaded.
fn settled_columns(render_distance: u32) -> usize {
    let r = f64::from(render_distance);
    (std::f64::consts::PI * r * r * 0.8) as usize
}
