//! `strata --texture-sheet <png>`: every texture side by side, enlarged, above a render of
//! the preview diorama, to judge the visual style at a glance.

use crate::camera::Camera;
use crate::render::MeshData;
use crate::screenshot::{ScreenshotError, render_image, write_png};
use glam::DVec3;
use std::path::Path;
use strata_core::content::{Content, TextureId, upscale};
use strata_core::texture::{Rgba, RgbaImage};

const TILE: u32 = 64;
const GAP: u32 = 12;
const COLUMNS: u32 = 12;
const BACKGROUND: Rgba = Rgba([0x26, 0x26, 0x2b, 255]);
const SCENE_SIZE: (u32, u32) = (TILE * COLUMNS + GAP * (COLUMNS - 1), 540);

pub fn write_texture_sheet(
    content: &Content,
    preview: &MeshData,
    path: &Path,
) -> Result<(), ScreenshotError> {
    let tiles: Vec<RgbaImage> = sheet_order(content)
        .into_iter()
        .filter_map(|id| content.textures.images.get(id))
        .map(|image| upscale(image, TILE))
        .collect();
    let rows = (tiles.len() as u32).div_ceil(COLUMNS);
    let width = SCENE_SIZE.0 + 2 * GAP;
    let height = GAP + rows * (TILE + GAP) + SCENE_SIZE.1 + GAP;
    let mut sheet = RgbaImage::new(width, height, BACKGROUND);
    for (index, tile) in tiles.iter().enumerate() {
        let (column, row) = (index as u32 % COLUMNS, index as u32 / COLUMNS);
        sheet.draw(tile, GAP + column * (TILE + GAP), GAP + row * (TILE + GAP));
    }
    let scene = render_image(content, preview, &diorama_camera(), SCENE_SIZE)?;
    sheet.draw(&scene, GAP, GAP + rows * (TILE + GAP));
    write_png(path, &sheet)?;
    tracing::info!(path = %path.display(), textures = tiles.len(), "texture sheet saved");
    Ok(())
}

/// Block textures first, in block order, then the other textures (items, sprites).
fn sheet_order(content: &Content) -> Vec<TextureId> {
    let mut order: Vec<TextureId> = Vec::new();
    let block_faces = content.blocks.iter().flat_map(|(_, _, block)| block.faces);
    let others = content.textures.images.iter().map(|(id, _, _)| id);
    for id in block_faces.chain(others) {
        if id != TextureId::MISSING && !order.contains(&id) {
            order.push(id);
        }
    }
    order
}

fn diorama_camera() -> Camera {
    Camera::new(DVec3::new(-12.0, 15.5, 14.0), 41.0, -28.0, 60.0)
}
