//! What the menus need from the program around the client: the saved worlds and a way
//! to start a server for one. The client never touches the files itself.

use crate::world::ClientWorld;
use glam::DVec3;
use std::thread::JoinHandle;
use strata_protocol::{ClientConnection, NewWorld, WorldSummary};

/// A world being played: the connection to its server and the server's thread.
pub struct RunningWorld {
    pub connection: ClientConnection,
    pub server: JoinHandle<()>,
}

/// Why a world could not be started; each maps to a message on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchError {
    /// Saved by another, incompatible version of the game.
    Incompatible,
    /// Missing, unreadable, or the disk refused to write.
    Unavailable,
}

impl LaunchError {
    pub fn translation_key(self) -> &'static str {
        match self {
            Self::Incompatible => "ui-error-incompatible",
            Self::Unavailable => "ui-error-unavailable",
        }
    }
}

/// Background of the title screen: a small generated world and where to look from.
pub struct Panorama {
    pub world: ClientWorld,
    pub center: DVec3,
}

pub trait Launcher {
    /// Saved worlds, most recently played first.
    fn worlds(&self) -> Vec<WorldSummary>;
    fn create(&mut self, world: NewWorld) -> Result<RunningWorld, LaunchError>;
    fn open(&mut self, id: &str) -> Result<RunningWorld, LaunchError>;
    fn delete(&mut self, id: &str) -> Result<(), LaunchError>;
    fn panorama(&self) -> Panorama;
}
