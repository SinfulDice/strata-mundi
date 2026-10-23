use super::atlas::UiAtlas;
use super::rect::Rect;
use bytemuck::{Pod, Zeroable};
use strata_core::texture::Rgba;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct UiVertex {
    /// Screen pixels.
    pub position: [f32; 2],
    /// Atlas pixels.
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

/// Collects the interface of one frame as textured quads, in interface pixels multiplied
/// by an integer scale so pixel art stays crisp.
pub struct Painter<'a> {
    pub atlas: &'a UiAtlas,
    pub scale: u32,
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
}

impl<'a> Painter<'a> {
    pub fn new(atlas: &'a UiAtlas, scale: u32) -> Self {
        Self {
            atlas,
            scale: scale.max(1),
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn fill(&mut self, rect: Rect, color: Rgba) {
        self.quad(rect, self.atlas.white, color);
    }

    /// One-pixel frame inside `rect`, lighter on top-left and darker on bottom-right.
    pub fn bevel(&mut self, rect: Rect, light: Rgba, dark: Rgba) {
        let Rect { x, y, w, h } = rect;
        self.fill(Rect::new(x, y, w, 1), light);
        self.fill(Rect::new(x, y, 1, h), light);
        self.fill(Rect::new(x, y + h as i32 - 1, w, 1), dark);
        self.fill(Rect::new(x + w as i32 - 1, y, 1, h), dark);
    }

    pub fn image(&mut self, dest: Rect, source: Rect) {
        self.quad(dest, source, Rgba([255; 4]));
    }

    /// Draws `text` with a drop shadow; returns its width.
    pub fn text(&mut self, x: i32, y: i32, text: &str, color: Rgba, shadow: Rgba) -> u32 {
        self.text_sized(x, y, text, color, shadow, 1)
    }

    /// `text` magnified `size` times (titles), shadow included.
    pub fn text_sized(
        &mut self,
        x: i32,
        y: i32,
        text: &str,
        color: Rgba,
        shadow: Rgba,
        size: u32,
    ) -> u32 {
        let offset = size as i32;
        self.text_plain(x + offset, y + offset, text, shadow, size);
        self.text_plain(x, y, text, color, size)
    }

    pub fn text_width(&self, text: &str) -> u32 {
        self.atlas.font.width(text)
    }

    pub fn line_height(&self) -> u32 {
        self.atlas.font.line_height
    }

    /// Glyph sizes are in atlas pixels, `density` per interface pixel: a dense font
    /// lands on whole screen pixels when the density matches the interface scale.
    fn text_plain(&mut self, x: i32, y: i32, text: &str, color: Rgba, size: u32) -> u32 {
        let step = size as f32 / self.atlas.font.density as f32;
        let mut pen = x as f32;
        for ch in text.chars() {
            let (source, advance) = self.atlas.glyph(ch);
            let (w, h) = (source.w as f32 * step, source.h as f32 * step);
            self.quad_at([pen, y as f32], [w, h], source, color);
            pen += advance as f32 * step;
        }
        (pen - x as f32).ceil().max(0.0) as u32
    }

    fn quad(&mut self, dest: Rect, source: Rect, color: Rgba) {
        let (origin, size) = (
            [dest.x as f32, dest.y as f32],
            [dest.w as f32, dest.h as f32],
        );
        self.quad_at(origin, size, source, color);
    }

    /// `origin` and `size` in interface pixels, possibly fractional.
    fn quad_at(&mut self, origin: [f32; 2], size: [f32; 2], source: Rect, color: Rgba) {
        let s = self.scale as f32;
        let (x0, y0) = ((origin[0] * s).round(), (origin[1] * s).round());
        let (x1, y1) = (x0 + (size[0] * s).round(), y0 + (size[1] * s).round());
        let (u0, v0) = (source.x as f32, source.y as f32);
        let (u1, v1) = (u0 + source.w as f32, v0 + source.h as f32);
        let base = self.vertices.len() as u32;
        for (position, uv) in [
            ([x0, y0], [u0, v0]),
            ([x1, y0], [u1, v0]),
            ([x1, y1], [u1, v1]),
            ([x0, y1], [u0, v1]),
        ] {
            self.vertices.push(UiVertex {
                position,
                uv,
                color: color.0,
            });
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Largest whole scale at which a 320 × 220 interface fits the window (Minecraft-like
/// "auto" interface scale), capped by the player's choice.
pub fn auto_scale(width: u32, height: u32, max: u32) -> u32 {
    (width / 320).min(height / 220).clamp(1, max.max(1))
}
