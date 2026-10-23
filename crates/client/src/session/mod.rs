mod apply;
mod transitions;

use crate::app::{ClientConfig, ClientError};
use crate::bench::BenchRun;
use crate::camera::Camera;
use crate::input::{Action, Binding, Gamepads, InputState};
use crate::launcher::Launcher;
use crate::mesher::BlockTable;
use crate::render::{
    Acquired, Gpu, PostProcess, RenderOptions, Renderer, Target, TextureFiltering, WindowSurface,
};
use crate::screens::{Screens, UiFrame};
use crate::settings::Settings;
use crate::sleep::SleepWatcher;
use crate::stage::Stage;
use glam::{DVec3, Vec2};
use std::sync::Arc;
use std::time::{Duration, Instant};
use strata_core::content::Content;
use strata_protocol::{MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE};
use winit::event::{DeviceEvent, MouseScrollDelta, WindowEvent};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Window};

const MAX_FRAME_SECONDS: f32 = 0.1;
/// Frames per second while the window is in the background (the world goes on).
const BACKGROUND_FPS: u32 = 5;
/// Pixels of trackpad scrolling counted as one wheel notch.
const PIXELS_PER_NOTCH: f64 = 40.0;

/// Everything that lives while the game window is open.
pub struct Session {
    pub gpu: Gpu,
    pub surface: WindowSurface,
    renderer: Renderer,
    post: PostProcess,
    screens: Screens,
    stage: Stage,
    launcher: Box<dyn Launcher>,
    content: Arc<Content>,
    table: Arc<BlockTable>,
    /// The settings in effect (the screens own the edited copy; changes are diffed).
    applied: Settings,
    bench: Option<BenchRun>,
    input: InputState,
    idle: InputState,
    gamepads: Gamepads,
    cursor_captured: bool,
    focused: bool,
    quit: bool,
    last_frame: Instant,
    sleep: Option<SleepWatcher>,
    /// The system is going to sleep (lid closed): the world stays paused.
    suspended: bool,
}

impl Session {
    pub fn new(window: Arc<Window>, config: ClientConfig) -> Result<Self, ClientError> {
        let instance = Gpu::create_instance();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(crate::render::GpuError::from)?;
        let gpu = Gpu::new(instance, Some(&surface))?;
        let surface = WindowSurface::new(&gpu, window, surface, config.settings.graphics.vsync);
        let graphics = &config.settings.graphics;
        let filtering = TextureFiltering {
            mipmaps: graphics.mipmaps,
            anisotropy: graphics.anisotropy.clamp(1, 16) as u16,
        };
        let mut renderer =
            Renderer::new(&gpu, surface.format(), &config.content.textures, filtering)?;
        let applied = config.settings.clone();
        renderer.ambient_occlusion = f32::from(u8::from(applied.graphics.ambient_occlusion));
        renderer.vertical_distance = applied.graphics.vertical_distance;
        renderer.options = RenderOptions::from_settings(&applied);
        let table = Arc::new(BlockTable::new(
            config.content.blocks.iter().map(|(_, _, b)| b),
        ));
        let bench = config.bench.map(|bench| BenchRun::new(bench, &gpu));
        let screens = Screens::new(
            &gpu,
            surface.format(),
            config.ui,
            &config.content,
            config.settings,
            config.settings_path,
        );
        let post = PostProcess::new(&gpu, surface.format());
        let mut session = Self {
            post,
            gpu,
            surface,
            renderer,
            screens,
            stage: Stage::Empty,
            launcher: config.launcher,
            content: config.content,
            table,
            input: input_for(&applied),
            idle: input_for(&applied),
            applied,
            bench,
            gamepads: Gamepads::connect(),
            cursor_captured: false,
            focused: true,
            quit: false,
            last_frame: Instant::now(),
            sleep: SleepWatcher::start(),
            suspended: false,
        };
        match config.start {
            Some(running) => session.enter_world(running)?,
            None => session.enter_title()?,
        }
        Ok(session)
    }

    pub fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) => self.surface.resize(&self.gpu, size.width, size.height),
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                if !focused {
                    self.input.release_all();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state.is_pressed();
                if let (PhysicalKey::Code(code), Some(text), true) =
                    (event.physical_key, &event.text, pressed)
                {
                    self.screens.learn_key(code, text);
                    self.input.add_text(text);
                }
                if let (PhysicalKey::Code(code), false) = (event.physical_key, event.repeat) {
                    self.input.set_binding(Binding::Key(code), pressed);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.input
                    .set_binding(Binding::Mouse(button), state.is_pressed());
            }
            WindowEvent::MouseWheel { delta, .. } => self.input.add_scroll(match delta {
                MouseScrollDelta::LineDelta(_, y) => y,
                MouseScrollDelta::PixelDelta(p) => (p.y / PIXELS_PER_NOTCH) as f32,
            }),
            WindowEvent::CursorMoved { position, .. } => {
                self.input.set_cursor((position.x, position.y));
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

    /// When the next frame may start: `None` for "now" (no limit).
    pub fn next_frame_at(&self) -> Option<Instant> {
        let idle = !self.focused && self.applied.game.idle_when_unfocused && self.bench.is_none();
        let fps = match self.applied.graphics.fps_limit {
            _ if idle => BACKGROUND_FPS,
            0 => return None,
            limit => limit,
        };
        Some(self.last_frame + Duration::from_secs_f64(1.0 / f64::from(fps)))
    }

    /// Updates and renders one frame. Returns `false` when the client should quit.
    pub fn draw(&mut self) -> Result<bool, ClientError> {
        let start = Instant::now();
        let frame_seconds = (start - self.last_frame).as_secs_f32();
        self.last_frame = start;
        self.update(frame_seconds)?;
        let cpu_seconds = self.render();
        if let (Some(bench), Stage::Playing(world)) = (&mut self.bench, &self.stage) {
            if let Some(cpu_seconds) = cpu_seconds {
                let (frame_ms, cpu_ms) = (frame_seconds * 1000.0, cpu_seconds * 1000.0);
                bench.record_frame(&self.gpu, frame_ms, cpu_ms, world.server_tick);
            }
            if bench.is_finished() {
                let path = bench
                    .write_report(&self.gpu, self.surface.size(), self.surface.present_mode())
                    .map_err(ClientError::Report)?;
                tracing::info!(path = %path.display(), "performance report written");
                return Ok(false);
            }
        }
        Ok(!self.quit)
    }

    fn update(&mut self, frame_seconds: f32) -> Result<(), ClientError> {
        if self.focused {
            self.gamepads.poll(&mut self.input);
        }
        self.receive_messages()?;
        self.watch_sleep();
        let game = match &mut self.stage {
            Stage::Playing(world) => Some(&mut world.game),
            _ => None,
        };
        let handled = self.screens.handle(&self.input, game, &self.content);
        for action in handled.actions {
            self.perform(action)?;
        }
        self.apply_settings();
        let menu_open = self.screens.is_menu_open();
        let playing = matches!(self.stage, Stage::Playing(_));
        self.set_cursor_captured(playing && !menu_open && self.focused);
        if !handled.used {
            self.handle_system_actions();
        }
        self.advance_stage(frame_seconds, menu_open);
        self.input.end_frame();
        Ok(())
    }

    fn advance_stage(&mut self, frame_seconds: f32, menu_open: bool) {
        let pause = self.screens.wants_pause() || self.suspended;
        let (view, camera) = match &mut self.stage {
            Stage::Empty => return,
            Stage::Title(title) => {
                title.update(frame_seconds);
                (&mut title.view, title.camera.position)
            }
            Stage::Playing(world) => {
                world.set_paused(pause);
                let wanted = settled_columns(self.applied.graphics.render_distance);
                let settled = world.game.view.is_settled(wanted);
                let dt = frame_seconds.min(MAX_FRAME_SECONDS);
                match &mut self.bench {
                    // The bench starts once the initial world is loaded, so loading is not measured.
                    Some(bench) if settled || bench.started() => {
                        bench.advance(frame_seconds, &mut world.game.camera);
                        world.game.follow_camera(&world.connection);
                    }
                    Some(_) => {}
                    // In menus the world goes on (unless paused) but the player stands still.
                    None if menu_open => world.game.update(dt, &self.idle, &world.connection),
                    None => world.game.update(dt, &self.input, &world.connection),
                }
                world.game.sync_hotbar(&world.connection);
                self.renderer.outline = world.game.interaction.target.map(|hit| hit.block);
                (&mut world.game.view, world.game.camera.position)
            }
        };
        view.update(
            &self.gpu,
            &mut self.renderer,
            camera,
            self.applied.graphics.vertical_distance,
        );
    }

    fn handle_system_actions(&mut self) {
        if self.input.just_pressed(Action::RenderDistanceDown) {
            self.set_render_distance(self.applied.graphics.render_distance.saturating_sub(1));
        }
        if self.input.just_pressed(Action::RenderDistanceUp) {
            self.set_render_distance(self.applied.graphics.render_distance + 1);
        }
    }

    /// Changes how far the world is loaded, while playing.
    pub fn set_render_distance(&mut self, distance: u32) {
        let distance = distance.clamp(MIN_VIEW_DISTANCE, MAX_VIEW_DISTANCE);
        tracing::info!(distance, "render distance");
        self.screens.change_settings(|s| {
            s.graphics.render_distance = distance;
            s.preset = None;
        });
    }

    fn camera(&self) -> Camera {
        match &self.stage {
            Stage::Title(title) => title.camera.clone(),
            Stage::Playing(world) => world.game.camera.clone(),
            Stage::Empty => Camera::new(DVec3::ZERO, 0.0, 0.0, self.applied.graphics.fov as f32),
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
        self.render_world(&mut encoder, &view);
        self.draw_interface(&mut encoder, &view);
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

    /// The 3D image, straight into the frame or through the final pass when the render
    /// scale or a colour filter needs it.
    fn render_world(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let camera = self.camera();
        let timestamps = self.bench.as_mut().and_then(BenchRun::timestamp_writes);
        let options = self.renderer.options;
        let size = self.surface.size();
        if !options.needs_post() {
            let target = Target { view, size };
            self.renderer
                .render(&self.gpu, encoder, target, &camera, timestamps);
            return;
        }
        let scaled = size.0 as f32 * options.render_scale;
        let scene_size = (scaled as u32, (size.1 as f32 * options.render_scale) as u32);
        self.post.prepare(&self.gpu, scene_size);
        if let Some(scene) = self.post.scene_view() {
            let target = Target {
                view: scene,
                size: (scene_size.0.max(1), scene_size.1.max(1)),
            };
            self.renderer
                .render(&self.gpu, encoder, target, &camera, timestamps);
        }
        self.post
            .draw(&self.gpu, encoder, view, options.color_filter);
    }

    fn draw_interface(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let game = match &mut self.stage {
            Stage::Playing(world) => Some(&mut world.game),
            _ => None,
        };
        let frame = UiFrame {
            encoder,
            view,
            size: self.surface.size(),
        };
        self.screens.draw(&self.gpu, frame, &self.content, game);
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

fn input_for(settings: &Settings) -> InputState {
    let mut input = InputState::new(settings.controls.bindings.clone());
    input.configure(
        &settings.controls.bindings,
        settings.controls.dead_zone_fraction(),
    );
    input
}
