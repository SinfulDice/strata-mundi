//! Textures made of several images: animation frames, random variants and the borders
//! connected textures take off between identical neighbours.

use super::recipe::{BASE_SIZE, TextureRecipe};
use super::{Rgba, RgbaImage};
use crate::id::Identifier;
use serde::Deserialize;

/// Images of one texture at most (frames or variants).
pub const MAX_IMAGES: u32 = 64;
/// Frame time of a PNG strip that replaces no animated texture, in ticks.
pub const DEFAULT_FRAME_TIME: u32 = 2;
pub const MAX_FRAME_TIME: u32 = 255;

/// How the images of an animation or of random variants are made.
#[derive(Debug, Clone, Deserialize)]
pub enum Frames {
    /// One image per listed texture (PNG or recipe).
    List(Vec<Identifier>),
    /// `count` drawings of `recipe`, each with its own seed (noise, ores).
    Reseed {
        recipe: Box<TextureRecipe>,
        count: u32,
    },
    /// `base` moved by `step` pixels (at 16 px) per image, wrapping around.
    Scroll {
        base: Identifier,
        count: u32,
        step: (i32, i32),
    },
    /// Rows of `base` pushed sideways along a wave that goes round once (flames, water).
    Sway {
        base: Identifier,
        count: u32,
        #[serde(default = "one")]
        amplitude: u32,
    },
}

/// Which neighbours a connected texture joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Connection {
    /// The four sides of every face (glass): 16 images, bits up 1, right 2, down 4, left 8.
    Edges,
    /// Above and below, on side faces only (bookshelves): 4 images, bits up 1, down 2.
    Vertical,
}

impl Connection {
    pub fn images(self) -> u32 {
        match self {
            Self::Edges => 16,
            Self::Vertical => 4,
        }
    }

    /// Image sides joined by variant `mask`, as bits up 1, right 2, down 4, left 8.
    fn sides(self, mask: u32) -> u32 {
        match self {
            Self::Edges => mask,
            Self::Vertical => (mask & 1) | ((mask & 2) << 1),
        }
    }
}

/// How the images of a texture are used; the first image is the texture's own layer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    Still,
    /// Played in a loop by the shader, `frame_time` ticks each.
    Animated { frame_time: u32 },
    /// Picked per block position; one weight per image.
    Random { weights: Vec<u32> },
    /// Picked by the mesher from the neighbours, `Connection::images` images.
    Connected(Connection),
}

impl Frames {
    pub fn count(&self) -> u32 {
        match self {
            Self::List(ids) => ids.len() as u32,
            Self::Reseed { count, .. } | Self::Scroll { count, .. } | Self::Sway { count, .. } => {
                *count
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let count = self.count();
        if count == 0 || count > MAX_IMAGES {
            return Err(format!("{count} images; expected 1 to {MAX_IMAGES}"));
        }
        match self {
            Self::Sway { amplitude, .. } if *amplitude > BASE_SIZE / 2 => Err(format!(
                "`amplitude` is {amplitude}, the maximum is {}",
                BASE_SIZE / 2
            )),
            Self::Reseed { recipe, .. } if recipe.is_multi() => {
                Err("a reseeded recipe cannot have several images itself".into())
            }
            _ => Ok(()),
        }
    }
}

/// Splits a vertical strip of square frames (PNG animations, like Minecraft).
pub fn split_strip(strip: &RgbaImage) -> Vec<RgbaImage> {
    let size = strip.width();
    (0..strip.height() / size.max(1))
        .map(|frame| {
            let mut image = RgbaImage::new(size, size, Rgba([0; 4]));
            for y in 0..size {
                for x in 0..size {
                    image.set(x, y, strip.get(x, frame * size + y));
                }
            }
            image
        })
        .collect()
}

/// `image` moved by `(dx, dy)` sixteenths of its size, wrapping around.
pub fn scrolled(image: &RgbaImage, (dx, dy): (i32, i32)) -> RgbaImage {
    let size = image.width();
    let scale = (size / BASE_SIZE).max(1) as i32;
    let wrap = |c: i32| c.rem_euclid(size as i32) as u32;
    shifted(image, |x, y| {
        (wrap(x as i32 - dx * scale), wrap(y as i32 - dy * scale))
    })
}

/// Frame `frame` of `count` of a sideways wave of `amplitude` sixteenths.
pub fn swayed(image: &RgbaImage, frame: u32, count: u32, amplitude: u32) -> RgbaImage {
    let size = image.width();
    let scale = (size / BASE_SIZE).max(1);
    let turn = std::f32::consts::TAU;
    shifted(image, |x, y| {
        let phase = frame as f32 / count.max(1) as f32 + (y / scale) as f32 / BASE_SIZE as f32;
        let offset = ((phase * turn).sin() * amplitude as f32).round() as i32 * scale as i32;
        ((x as i32 - offset).rem_euclid(size as i32) as u32, y)
    })
}

fn shifted(image: &RgbaImage, source: impl Fn(u32, u32) -> (u32, u32)) -> RgbaImage {
    let mut out = RgbaImage::new(image.width(), image.height(), Rgba([0; 4]));
    for y in 0..image.height() {
        for x in 0..image.width() {
            let (sx, sy) = source(x, y);
            out.set(x, y, image.get(sx, sy));
        }
    }
    out
}

/// Every image of a connected texture: the borders of joined sides are replaced by the
/// band just inside them, so two neighbours read as one surface.
pub fn connected(base: &RgbaImage, connect: Connection, border: u32) -> Vec<RgbaImage> {
    let band = (border * base.width() / BASE_SIZE).clamp(1, base.width() / 2);
    (0..connect.images())
        .map(|mask| trimmed(base, connect.sides(mask), band))
        .collect()
}

/// Sides in the order of the mask bits: up, right, down, left. Applied one after the
/// other, so a corner between two joined sides opens up too.
fn trimmed(base: &RgbaImage, sides: u32, band: u32) -> RgbaImage {
    let mut image = base.clone();
    let last = base.width() - 1;
    for side in 0..4 {
        if sides & (1 << side) == 0 {
            continue;
        }
        let source = image.clone();
        for i in 0..band {
            for j in 0..=last {
                let (to, from) = match side {
                    0 => ((j, i), (j, i + band)),
                    1 => ((last - i, j), (last - i - band, j)),
                    2 => ((j, last - i), (j, last - i - band)),
                    _ => ((i, j), (i + band, j)),
                };
                image.set(to.0, to.1, source.get(from.0, from.1));
            }
        }
    }
    image
}

fn one() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed() -> RgbaImage {
        let mut image = RgbaImage::new(16, 16, Rgba([0; 4]));
        for i in 0..16 {
            for (x, y) in [(i, 0), (i, 15), (0, i), (15, i)] {
                image.set(x, y, Rgba([255; 4]));
            }
        }
        image
    }

    #[test]
    fn joined_sides_lose_their_border() {
        let images = connected(&framed(), Connection::Edges, 1);
        assert_eq!(images.len(), 16);
        assert_eq!(images[0], framed(), "alone: the whole frame");
        let right = &images[2];
        assert_eq!(right.get(15, 8), Rgba([0; 4]));
        assert_eq!(right.get(0, 8), Rgba([255; 4]));
        assert_eq!(
            right.get(15, 0),
            Rgba([255; 4]),
            "top border runs to the edge"
        );
        let all = &images[15];
        assert_eq!(
            all.get(0, 0),
            Rgba([0; 4]),
            "corners open between joined sides"
        );
    }

    #[test]
    fn vertical_connections_trim_top_and_bottom_only() {
        let images = connected(&framed(), Connection::Vertical, 1);
        assert_eq!(images.len(), 4);
        assert_eq!(images[1].get(8, 0), Rgba([0; 4]));
        assert_eq!(images[1].get(8, 15), Rgba([255; 4]));
        assert_eq!(images[2].get(8, 15), Rgba([0; 4]));
        assert_eq!(images[3].get(0, 8), Rgba([255; 4]), "sides stay framed");
    }

    #[test]
    fn scrolling_and_swaying_wrap_around() {
        let mut image = RgbaImage::new(16, 16, Rgba([0; 4]));
        image.set(15, 15, Rgba([9; 4]));
        assert_eq!(scrolled(&image, (1, 1)).get(0, 0), Rgba([9; 4]));
        let frames: Vec<_> = (0..4).map(|f| swayed(&framed(), f, 4, 1)).collect();
        assert_ne!(frames[0], frames[1]);
        assert_eq!(swayed(&framed(), 4, 4, 1), frames[0], "the wave loops");
    }

    #[test]
    fn strips_split_into_square_frames() {
        let mut strip = RgbaImage::new(16, 48, Rgba([0; 4]));
        strip.set(0, 32, Rgba([7; 4]));
        let frames = split_strip(&strip);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[2].get(0, 0), Rgba([7; 4]));
    }
}
