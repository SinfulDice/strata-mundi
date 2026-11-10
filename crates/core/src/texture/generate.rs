use super::color::Palette;
use super::multi::{
    DEFAULT_FRAME_TIME, Frames, Layout, MAX_IMAGES, connected, scrolled, split_strip, swayed,
};
use super::recipe::{Painter, TextureRecipe};
use super::{Rgba, RgbaImage};
use crate::id::Identifier;
use std::collections::BTreeMap;

/// Where a texture comes from once all packs are layered.
pub enum Source<'a> {
    Recipe(&'a TextureRecipe),
    Image(&'a RgbaImage),
}

/// Deeper chains are certainly a reference loop (`a` drawn on `b` drawn on `a`).
const MAX_DEPTH: u32 = 8;

/// The images of one texture, first image first, and how they are used.
#[derive(Debug, Clone)]
pub struct Generated {
    pub images: Vec<RgbaImage>,
    pub layout: Layout,
}

impl Generated {
    fn still(image: RgbaImage) -> Self {
        Self {
            images: vec![image],
            layout: Layout::Still,
        }
    }

    fn first(mut self) -> RgbaImage {
        self.images.swap_remove(0)
    }
}

struct Context<'a, 'b> {
    lookup: &'b dyn Fn(&Identifier) -> Option<Source<'a>>,
    palette: &'b Palette,
}

/// Draws texture `id`, following its bases and recolors. Bases always use the first image
/// of the texture they name.
pub fn generate<'a>(
    id: &Identifier,
    lookup: &dyn Fn(&Identifier) -> Option<Source<'a>>,
    palette: &Palette,
) -> Result<Generated, String> {
    let context = Context { lookup, palette };
    let generated = context.texture(id, &BTreeMap::new(), 0)?;
    if generated.images.len() > MAX_IMAGES as usize {
        return Err(format!(
            "{} images; a texture has {MAX_IMAGES} at most",
            generated.images.len()
        ));
    }
    Ok(generated)
}

impl Context<'_, '_> {
    fn texture(
        &self,
        id: &Identifier,
        swap: &BTreeMap<String, String>,
        depth: u32,
    ) -> Result<Generated, String> {
        if depth > MAX_DEPTH {
            return Err(format!("texture `{id}` is part of a reference loop"));
        }
        match (self.lookup)(id).ok_or_else(|| format!("unknown texture `{id}`"))? {
            Source::Image(image) => Ok(from_image(image)),
            Source::Recipe(recipe) => self.recipe(recipe, seed_of(id), swap, depth),
        }
    }

    fn base(
        &self,
        id: &Identifier,
        swap: &BTreeMap<String, String>,
        depth: u32,
    ) -> Result<RgbaImage, String> {
        Ok(self.texture(id, swap, depth + 1)?.first())
    }

    fn recipe(
        &self,
        recipe: &TextureRecipe,
        seed: u64,
        swap: &BTreeMap<String, String>,
        depth: u32,
    ) -> Result<Generated, String> {
        recipe.validate()?;
        match recipe {
            TextureRecipe::Recolor { base, swap: inner } => {
                self.texture(base, &compose(inner, swap), depth + 1)
            }
            TextureRecipe::Animated { frames, frame_time } => Ok(Generated {
                images: self.frames(frames, seed, swap, depth)?,
                layout: Layout::Animated {
                    frame_time: *frame_time,
                },
            }),
            TextureRecipe::Random { variants, weights } => {
                let images = self.frames(variants, seed, swap, depth)?;
                let weights = if weights.is_empty() {
                    vec![1; images.len()]
                } else {
                    weights.clone()
                };
                if weights.len() != images.len() {
                    return Err(format!(
                        "{} weights for {} variants",
                        weights.len(),
                        images.len()
                    ));
                }
                Ok(Generated {
                    images,
                    layout: Layout::Random { weights },
                })
            }
            TextureRecipe::Connected {
                base,
                connect,
                border,
            } => Ok(Generated {
                images: connected(&self.base(base, swap, depth)?, *connect, *border),
                layout: Layout::Connected(*connect),
            }),
            _ => self.paint(recipe, seed, swap, depth).map(Generated::still),
        }
    }

    fn paint(
        &self,
        recipe: &TextureRecipe,
        seed: u64,
        swap: &BTreeMap<String, String>,
        depth: u32,
    ) -> Result<RgbaImage, String> {
        let painter = Painter {
            palette: self.palette,
            swap,
            seed,
        };
        let layer = recipe.paint_layer(&painter)?;
        let Some(base) = recipe.base() else {
            return Ok(layer);
        };
        let mut image = self.base(base, swap, depth)?;
        image.draw(&upscale(&layer, image.width()), 0, 0);
        Ok(image)
    }

    fn frames(
        &self,
        frames: &Frames,
        seed: u64,
        swap: &BTreeMap<String, String>,
        depth: u32,
    ) -> Result<Vec<RgbaImage>, String> {
        Ok(match frames {
            Frames::List(ids) => {
                let mut images = Vec::new();
                for id in ids {
                    let generated = self.texture(id, swap, depth + 1)?;
                    if generated.images.len() > 1 {
                        return Err(format!(
                            "`{id}` has {} images; `List` takes textures of one image",
                            generated.images.len()
                        ));
                    }
                    images.push(generated.first());
                }
                images
            }
            Frames::Reseed { recipe, count } => (0..*count)
                .map(|k| {
                    let seed = seed.wrapping_add(u64::from(k).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                    Ok(self.recipe(recipe, seed, swap, depth + 1)?.first())
                })
                .collect::<Result<_, String>>()?,
            Frames::Scroll { base, count, step } => {
                let base = self.base(base, swap, depth)?;
                (0..*count as i32)
                    .map(|k| scrolled(&base, (step.0 * k, step.1 * k)))
                    .collect()
            }
            Frames::Sway {
                base,
                count,
                amplitude,
            } => {
                let base = self.base(base, swap, depth)?;
                (0..*count)
                    .map(|k| swayed(&base, k, *count, *amplitude))
                    .collect()
            }
        })
    }
}

/// A square PNG, or a vertical strip of square frames played as an animation.
fn from_image(image: &RgbaImage) -> Generated {
    if image.height() == image.width() {
        return Generated::still(image.clone());
    }
    Generated {
        images: split_strip(image),
        layout: Layout::Animated {
            frame_time: DEFAULT_FRAME_TIME,
        },
    }
}

/// Nearest-neighbour upscale to `size` (a multiple of the image size), keeping pixel art crisp.
pub fn upscale(image: &RgbaImage, size: u32) -> RgbaImage {
    if image.width() == size {
        return image.clone();
    }
    let factor = (size / image.width().max(1)).max(1);
    let mut scaled = RgbaImage::new(size, size, Rgba([0; 4]));
    for y in 0..size {
        for x in 0..size {
            let source = (
                (x / factor).min(image.width() - 1),
                (y / factor).min(image.height() - 1),
            );
            scaled.set(x, y, image.get(source.0, source.1));
        }
    }
    scaled
}

/// `inner` renames first, then `outer` renames the result.
fn compose(
    inner: &BTreeMap<String, String>,
    outer: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut combined = outer.clone();
    for (from, to) in inner {
        combined.insert(from.clone(), outer.get(to).unwrap_or(to).clone());
    }
    combined
}

/// Stable per-texture seed, so two textures with the same recipe still differ.
pub fn seed_of(id: &Identifier) -> u64 {
    id.to_string()
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325, |hash: u64, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_list_refuses_textures_of_several_images() {
        let strip = RgbaImage::new(16, 32, Rgba([255; 4]));
        let square = RgbaImage::new(16, 16, Rgba([255; 4]));
        let ids = |names: &[&str]| names.iter().map(|n| n.parse().unwrap()).collect();
        let recipe = |names: &[&str]| TextureRecipe::Animated {
            frames: Frames::List(ids(names)),
            frame_time: 2,
        };
        let (good, bad) = (recipe(&["t:a", "t:a"]), recipe(&["t:a", "t:strip"]));
        let lookup = |id: &Identifier| match id.to_string().as_str() {
            "t:a" => Some(Source::Image(&square)),
            "t:strip" => Some(Source::Image(&strip)),
            "t:good" => Some(Source::Recipe(&good)),
            "t:bad" => Some(Source::Recipe(&bad)),
            _ => None,
        };
        let palette = Palette::default();
        let good = generate(&"t:good".parse().unwrap(), &lookup, &palette).unwrap();
        assert_eq!(good.images.len(), 2);
        let error = generate(&"t:bad".parse().unwrap(), &lookup, &palette).unwrap_err();
        assert!(error.contains("t:strip"), "{error}");
    }
}
