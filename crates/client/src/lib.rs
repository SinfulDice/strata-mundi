//! Client side: renders what the server sends and turns player input into intents.

mod app;
pub mod bench;
pub mod camera;
mod fly;
pub mod input;
pub mod preview;
pub mod render;
pub mod screenshot;
mod session;
pub mod settings;
pub mod texture_sheet;

pub use app::{ClientConfig, ClientError, run};
