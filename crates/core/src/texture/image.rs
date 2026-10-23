#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }
}

/// RGBA image stored as raw bytes, rows top to bottom, ready for GPU upload or PNG encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

impl RgbaImage {
    pub fn new(width: u32, height: u32, fill: Rgba) -> Self {
        Self {
            width,
            height,
            bytes: fill.0.repeat((width * height) as usize),
        }
    }

    /// `bytes` must hold exactly `width * height` RGBA pixels, rows top to bottom.
    pub fn from_bytes(width: u32, height: u32, bytes: Vec<u8>) -> Option<Self> {
        (bytes.len() == (width * height * 4) as usize).then_some(Self {
            width,
            height,
            bytes,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get(&self, x: u32, y: u32) -> Rgba {
        let start = self.offset(x, y);
        let mut pixel = [0; 4];
        pixel.copy_from_slice(&self.bytes[start..start + 4]);
        Rgba(pixel)
    }

    pub fn set(&mut self, x: u32, y: u32, color: Rgba) {
        let start = self.offset(x, y);
        self.bytes[start..start + 4].copy_from_slice(&color.0);
    }

    /// Draws `source` with its top-left corner at `(x, y)`, alpha-blended, clipped to bounds.
    pub fn draw(&mut self, source: &RgbaImage, x: u32, y: u32) {
        for sy in 0..source.height.min(self.height.saturating_sub(y)) {
            for sx in 0..source.width.min(self.width.saturating_sub(x)) {
                let over = source.get(sx, sy);
                let under = self.get(x + sx, y + sy);
                self.set(x + sx, y + sy, blend(over, under));
            }
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn offset(&self, x: u32, y: u32) -> usize {
        debug_assert!(x < self.width && y < self.height);
        (y * self.width + x) as usize * 4
    }
}

/// `over` on top of an opaque `under`.
fn blend(over: Rgba, under: Rgba) -> Rgba {
    let alpha = u16::from(over.0[3]);
    let mix = |o: u8, u: u8| ((u16::from(o) * alpha + u16::from(u) * (255 - alpha)) / 255) as u8;
    Rgba([
        mix(over.0[0], under.0[0]),
        mix(over.0[1], under.0[1]),
        mix(over.0[2], under.0[2]),
        under.0[3].max(over.0[3]),
    ])
}
