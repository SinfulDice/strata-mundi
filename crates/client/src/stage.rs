//! What the window shows under the interface: the title panorama or a world being played.

use crate::camera::Camera;
use crate::gameplay::Gameplay;
use crate::launcher::RunningWorld;
use crate::streaming::WorldView;
use glam::DVec3;
use std::thread::JoinHandle;
use strata_protocol::{ClientConnection, ClientMessage};

/// Turns of the panorama per minute: slow enough to read the menus over it.
const PANORAMA_TURNS_PER_MINUTE: f32 = 0.5;
const PANORAMA_HEIGHT: f64 = 22.0;
const PANORAMA_PITCH: f32 = -0.25;

pub enum Stage {
    /// Between two stages (a world being saved, the panorama being generated).
    Empty,
    Title(Box<TitleScene>),
    Playing(Box<InWorld>),
}

pub struct TitleScene {
    pub view: WorldView,
    pub camera: Camera,
    center: DVec3,
    angle: f32,
}

impl TitleScene {
    pub fn new(view: WorldView, center: DVec3, fov: f32) -> Self {
        let camera = Camera::new(center, 0.0, PANORAMA_PITCH, fov);
        Self {
            view,
            camera,
            center,
            angle: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.angle += dt * PANORAMA_TURNS_PER_MINUTE * std::f32::consts::TAU / 60.0;
        self.camera.yaw = self.angle;
        self.camera.pitch = PANORAMA_PITCH;
        self.camera.position = self.center + DVec3::Y * PANORAMA_HEIGHT;
    }
}

pub struct InWorld {
    pub connection: ClientConnection,
    server: Option<JoinHandle<()>>,
    pub game: Gameplay,
    pub server_tick: u64,
    /// Last pause state sent to the server.
    pub paused: bool,
}

impl InWorld {
    pub fn new(running: RunningWorld, game: Gameplay) -> Self {
        Self {
            connection: running.connection,
            server: Some(running.server),
            game,
            server_tick: 0,
            paused: false,
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        if paused != self.paused {
            self.connection.send(ClientMessage::Pause(paused));
            self.paused = paused;
        }
    }

    /// Saves (with the capture for the world list), leaves, and waits until the server
    /// has written everything.
    pub fn leave(mut self, icon: Option<Vec<u8>>) {
        self.connection.send(ClientMessage::Save { icon });
        self.connection.send(ClientMessage::Disconnect);
        if let Some(server) = self.server.take()
            && server.join().is_err()
        {
            tracing::error!("the server thread panicked");
        }
    }
}
