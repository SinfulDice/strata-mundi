//! Deterministic procedural textures, generated on the CPU and uploaded by the client.

mod image;
mod material;
mod noise;

pub use image::{Rgba, RgbaImage};
pub use material::{MaterialRecipe, generate_material};
