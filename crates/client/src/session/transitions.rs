//! Moving between the title screen and worlds, and what a world sends back.

use super::Session;
use crate::app::ClientError;
use crate::camera::Camera;
use crate::gameplay::Gameplay;
use crate::interaction::BlockInfo;
use crate::launcher::RunningWorld;
use crate::mesh_scheduler::MeshScheduler;
use crate::render::{OffscreenTarget, Target};
use crate::screens::UiAction;
use crate::sleep::{SleepEvent, SleepWatcher};
use crate::stage::{InWorld, Stage, TitleScene};
use crate::streaming::WorldView;
use crate::ui::ICON_SIZE;
use crate::world::ClientWorld;
use glam::DVec3;
use std::sync::Arc;
use strata_protocol::{
    ClientMessage, Disconnected, MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE, ServerMessage,
};

impl Session {
    fn new_meshing(&self) -> Result<MeshScheduler, ClientError> {
        let graphics = &self.applied.graphics;
        MeshScheduler::new(
            Arc::clone(&self.table),
            graphics.meshing_threads(),
            graphics.ambient_occlusion,
        )
        .map_err(ClientError::Threads)
    }

    fn new_view(&self, world: ClientWorld) -> Result<WorldView, ClientError> {
        Ok(WorldView::new(world, self.new_meshing()?))
    }
    /// Meshing restarted with the current settings, for the world on screen.
    pub(super) fn remesh(&mut self) -> Result<(), ClientError> {
        let meshing = self.new_meshing()?;
        let view = match &mut self.stage {
            Stage::Title(title) => &mut title.view,
            Stage::Playing(world) => &mut world.game.view,
            Stage::Empty => return Ok(()),
        };
        view.replace_meshing(meshing);
        Ok(())
    }

    pub(super) fn enter_title(&mut self) -> Result<(), ClientError> {
        self.renderer.clear();
        let panorama = self.launcher.panorama();
        let view = self.new_view(panorama.world)?;
        self.stage = Stage::Title(Box::new(TitleScene::new(
            view,
            panorama.center,
            self.applied.graphics.fov as f32,
        )));
        self.screens.show_title();
        Ok(())
    }
    pub(super) fn enter_world(&mut self, running: RunningWorld) -> Result<(), ClientError> {
        self.renderer.clear();
        let distance = self
            .applied
            .graphics
            .render_distance
            .clamp(MIN_VIEW_DISTANCE, MAX_VIEW_DISTANCE);
        running
            .connection
            .send(ClientMessage::ViewDistance(distance));
        let camera = Camera::new(DVec3::ZERO, 0.0, 0.0, self.applied.graphics.fov as f32);
        let view = self.new_view(ClientWorld::default())?;
        let info = BlockInfo::new(Arc::clone(&self.content));
        let mut game = Gameplay::new(view, camera, info, self.applied.controls.clone());
        game.camera_effects = !self.applied.interface.reduce_camera_effects;
        self.stage = Stage::Playing(Box::new(InWorld::new(running, game)));
        self.screens.enter_world();
        Ok(())
    }
    /// Saves and closes the world being played, if any (also when the window closes).
    pub fn leave_world(&mut self) {
        let icon = self.capture_icon();
        if let Stage::Playing(world) = std::mem::replace(&mut self.stage, Stage::Empty) {
            world.leave(icon);
        }
        self.renderer.clear();
    }
    pub(super) fn perform(&mut self, action: UiAction) -> Result<(), ClientError> {
        let started = match action {
            UiAction::ShowWorlds => {
                self.screens.open_worlds(&self.gpu, self.launcher.worlds());
                return Ok(());
            }
            UiAction::Play(id) => self.launcher.open(&id),
            UiAction::Create(world) => self.launcher.create(world),
            UiAction::Delete(id) => {
                match self.launcher.delete(&id) {
                    Ok(()) => self.screens.open_worlds(&self.gpu, self.launcher.worlds()),
                    Err(error) => self.screens.show_notice(error.translation_key()),
                }
                return Ok(());
            }
            UiAction::SaveAndQuit => {
                self.leave_world();
                return self.enter_title();
            }
            UiAction::Quit => {
                self.quit = true;
                return Ok(());
            }
        };
        match started {
            Ok(running) => self.enter_world(running),
            Err(error) => {
                self.screens.show_notice(error.translation_key());
                Ok(())
            }
        }
    }
    /// Lid closed: pause and save at once (with a capture), let the system sleep once the
    /// server confirms; lid opened: play on.
    pub(super) fn watch_sleep(&mut self) {
        let Some(event) = self.sleep.as_ref().and_then(SleepWatcher::poll) else {
            return;
        };
        match event {
            SleepEvent::Suspending => {
                self.suspended = true;
                let icon = self.capture_icon();
                match &mut self.stage {
                    Stage::Playing(world) => {
                        world.set_paused(true);
                        world.connection.send(ClientMessage::Save { icon });
                    }
                    _ => {
                        if let Some(sleep) = &self.sleep {
                            sleep.ready();
                        }
                    }
                }
            }
            SleepEvent::Resumed => self.suspended = false,
        }
    }
    pub(super) fn receive_messages(&mut self) -> Result<(), ClientError> {
        let Stage::Playing(world) = &mut self.stage else {
            return Ok(());
        };
        loop {
            match world.connection.try_recv() {
                Ok(Some(ServerMessage::Tick { tick })) => world.server_tick = tick,
                Ok(Some(ServerMessage::Saved)) => {
                    if let Some(sleep) = self.sleep.as_ref().filter(|_| self.suspended) {
                        sleep.ready();
                    }
                }
                Ok(Some(message)) => {
                    if let (ServerMessage::Welcome { body, .. }, Some(bench)) =
                        (&message, &mut self.bench)
                    {
                        bench.set_origin(body.position);
                    }
                    world
                        .game
                        .apply(message, &mut self.renderer, &world.connection);
                }
                Ok(None) => return Ok(()),
                Err(Disconnected) => return Err(ClientError::ServerGone),
            }
        }
    }
    /// A small render of the current view for the world list, as PNG.
    fn capture_icon(&mut self) -> Option<Vec<u8>> {
        let Stage::Playing(world) = &self.stage else {
            return None;
        };
        let camera = world.game.camera.clone();
        let target = OffscreenTarget::with_format(&self.gpu, ICON_SIZE, self.surface.format());
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        let frame = Target {
            view: &target.view,
            size: target.size,
        };
        self.renderer
            .render(&self.gpu, &mut encoder, frame, &camera, None);
        let image = target
            .read(&self.gpu, encoder)
            .map_err(|error| tracing::warn!(%error, "world capture failed"))
            .ok()?;
        crate::screenshot::encode_png(&image)
    }
}
