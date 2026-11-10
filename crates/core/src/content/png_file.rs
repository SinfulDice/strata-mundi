use crate::texture::{MAX_IMAGES, Rgba, RgbaImage};
use std::io::Cursor;
use std::path::Path;

/// Texture sizes accepted from packs and resource packs.
pub const ACCEPTED_SIZES: [u32; 3] = [16, 32, 64];

/// Reads a square 16, 32 or 64 px PNG as RGBA, or a vertical strip of such squares
/// (animation frames, top to bottom).
pub fn read_texture_png(path: &Path) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| "image too large".to_string())?;
    let mut buffer = vec![0; size];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let frames = info.height / info.width.max(1);
    if !ACCEPTED_SIZES.contains(&info.width)
        || info.height % info.width != 0
        || !(1..=MAX_IMAGES).contains(&frames)
    {
        return Err(format!(
            "texture is {}×{} px; expected 16×16, 32×32 or 64×64, or a vertical strip of \
             {MAX_IMAGES} such squares at most",
            info.width, info.height
        ));
    }
    let channels = info.color_type.samples();
    let pixels = buffer[..info.buffer_size()]
        .chunks(info.line_size)
        .flat_map(|line| line[..info.width as usize * channels].chunks(channels))
        .flat_map(|p| to_rgba(p).0)
        .collect();
    RgbaImage::from_bytes(info.width, info.height, pixels)
        .ok_or_else(|| "unexpected pixel data size".to_string())
}

fn to_rgba(pixel: &[u8]) -> Rgba {
    match *pixel {
        [r, g, b, a] => Rgba([r, g, b, a]),
        [r, g, b] => Rgba([r, g, b, 255]),
        [l, a] => Rgba([l, l, l, a]),
        [l] => Rgba([l, l, l, 255]),
        _ => Rgba([255, 0, 255, 255]),
    }
}

/// Magenta and black checkerboard shown wherever a texture is missing or broken.
pub fn missing_texture(size: u32) -> RgbaImage {
    let mut image = RgbaImage::new(size, size, Rgba([0, 0, 0, 255]));
    let half = size / 2;
    for y in 0..size {
        for x in 0..size {
            if (x < half) == (y < half) {
                image.set(x, y, Rgba([255, 0, 255, 255]));
            }
        }
    }
    image
}
