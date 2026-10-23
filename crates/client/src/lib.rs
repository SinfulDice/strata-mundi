//! Client side: renders what the server sends and turns player input into intents.

mod app;
pub mod bench;
pub mod camera;
mod fly;
pub mod input;
pub mod render;
pub mod screenshot;
mod session;
pub mod settings;
mod test_scene;

pub use app::{ClientConfig, ClientError, run};
