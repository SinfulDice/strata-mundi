//! Client side: renders what the server sends and turns player input into intents.

mod app;
pub mod bench;
pub mod camera;
pub mod catalog;
mod cinematic;
mod debug_overlay;
pub mod entities;
mod gameplay;
mod hotbar;
pub mod input;
mod interaction;
pub mod launcher;
mod lighting;
mod look;
mod mesh_scheduler;
pub mod mesher;
pub mod models;
mod player;
pub mod preview;
pub mod render;
mod screens;
pub mod screenshot;
mod session;
pub mod settings;
pub mod sky;
mod sleep;
mod stage;
mod streaming;
pub mod texture_sheet;
pub mod ui;
pub mod world;

pub use app::{ClientConfig, ClientError, DEFAULT_FOV, run};
