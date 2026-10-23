//! Drawing one frame: the world (directly or through the final pass), then the interface.

use super::Session;
use crate::bench::BenchRun;
use crate::camera::Camera;
use crate::render::{Acquired, Target};
use crate::screens::UiFrame;
use crate::stage::Stage;
use glam::DVec3;
use std::time::Instant;

impl Session {
    pub(super) fn camera(&self) -> Camera {
        match &self.stage {
            Stage::Title(title) => title.camera.clone(),
            Stage::Playing(world) => world.game.camera.clone(),
            Stage::Empty => Camera::new(DVec3::ZERO, 0.0, 0.0, self.applied.graphics.fov as f32),
        }
    }

    /// Returns the CPU time spent building and submitting the frame, in seconds,
    /// or `None` when no frame could be drawn.
    pub(super) fn render(&mut self) -> Option<f32> {
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
    pub(super) fn render_world(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let camera = self.camera();
        let timestamps = self.bench.as_mut().and_then(BenchRun::timestamp_writes);
        let options = self.renderer.options;
        let size = self.surface.size();
        if !options.needs_post() {
            self.post.release();
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

    pub(super) fn draw_interface(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let debug = self.debug_lines();
        let game = match &mut self.stage {
            Stage::Playing(world) => Some(&mut world.game),
            _ => None,
        };
        let frame = UiFrame {
            encoder,
            view,
            size: self.surface.size(),
            debug: &debug,
        };
        self.screens.draw(&self.gpu, frame, &self.content, game);
    }
}
