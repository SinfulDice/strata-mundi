use super::png_file::{missing_texture, read_texture_png};
use super::registry::{Registry, TextureId};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, category_files, read_ron};
use crate::texture::{
    BASE_SIZE, Generated, Layout, Palette, PaletteFile, RgbaImage, Source, TextureRecipe, generate,
    upscale,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Every texture of the game at one common resolution, ready for the GPU texture array:
/// the first image of each texture, then the other images (frames, variants).
#[derive(Debug, Clone)]
pub struct TextureLibrary {
    /// First image of every texture; its key is its GPU layer.
    pub images: Registry<TextureId, RgbaImage>,
    /// Other images, GPU layers `images.len()` and up.
    pub extra: Vec<RgbaImage>,
    /// How each texture uses its images, indexed by `TextureId`.
    pub layouts: Vec<TextureLayout>,
    pub resolution: u32,
}

/// A texture's images: its own layer, then `images - 1` layers from `extra` on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureLayout {
    pub layout: Layout,
    pub extra: u32,
    pub images: u32,
}

/// Layers a GPU texture array can address with the 16-bit layer of a vertex.
pub const MAX_LAYERS: usize = 1 << 16;

enum Loaded {
    Recipe(TextureRecipe),
    /// A PNG, with the frame time of the animated texture it replaces.
    Image(RgbaImage, Option<u32>),
}

impl TextureLibrary {
    /// `textures/` of every pack, PNG over RON within a pack, later packs on top.
    pub fn load(packs: &PackSet, errors: &mut Vec<PackError>) -> Self {
        let palette = load_palette(packs, errors);
        let sources = layered_sources(packs, errors);
        let resolution = sources
            .values()
            .filter_map(|(loaded, _)| match loaded {
                Loaded::Image(image, _) => Some(image.width()),
                Loaded::Recipe(_) => None,
            })
            .fold(BASE_SIZE, u32::max);
        let lookup = |id: &Identifier| {
            sources.get(id).map(|(loaded, _)| match loaded {
                Loaded::Recipe(recipe) => Source::Recipe(recipe),
                Loaded::Image(image, _) => Source::Image(image),
            })
        };
        let mut library = Self {
            images: Registry::default(),
            extra: Vec::new(),
            layouts: vec![TextureLayout::still()],
            resolution,
        };
        library
            .images
            .insert(missing_id(), missing_texture(resolution));
        for (id, (loaded, path)) in &sources {
            let mut generated = generate(id, &lookup, &palette).unwrap_or_else(|message| {
                errors.push(PackError::invalid(path, message));
                Generated {
                    images: vec![missing_texture(BASE_SIZE)],
                    layout: Layout::Still,
                }
            });
            if let (Loaded::Image(_, Some(time)), Layout::Animated { frame_time }) =
                (loaded, &mut generated.layout)
            {
                *frame_time = *time;
            }
            if !library.add(id, generated) {
                errors.push(PackError::invalid(
                    path,
                    "too many texture layers (65 536 at most)",
                ));
            }
        }
        library
    }

    fn add(&mut self, id: &Identifier, generated: Generated) -> bool {
        let mut images = generated.images.into_iter();
        let first = images.next().unwrap_or_else(|| missing_texture(BASE_SIZE));
        let extra: Vec<_> = images
            .map(|image| upscale(&image, self.resolution))
            .collect();
        if self.layer_count() + 1 + extra.len() > MAX_LAYERS
            || self
                .images
                .insert(id.clone(), upscale(&first, self.resolution))
                .is_none()
        {
            return false;
        }
        self.layouts.push(TextureLayout {
            layout: generated.layout,
            extra: self.extra.len() as u32,
            images: 1 + extra.len() as u32,
        });
        self.extra.extend(extra);
        true
    }

    /// Layer of `id`, if such a texture exists.
    pub fn layer(&self, id: &Identifier) -> Option<TextureId> {
        self.images.key(id)
    }

    pub fn layout(&self, texture: TextureId) -> &TextureLayout {
        static STILL: TextureLayout = TextureLayout::still();
        self.layouts.get(usize::from(texture.0)).unwrap_or(&STILL)
    }

    /// Every image in GPU layer order.
    pub fn layers(&self) -> impl Iterator<Item = &RgbaImage> {
        self.images
            .iter()
            .map(|(_, _, image)| image)
            .chain(&self.extra)
    }

    pub fn layer_count(&self) -> usize {
        self.images.len() + self.extra.len()
    }

    /// Layer of image `index` of `texture` (0 is the texture's own layer).
    pub fn image_layer(&self, texture: TextureId, index: u32) -> TextureId {
        let layout = self.layout(texture);
        if index == 0 || index >= layout.images {
            return texture;
        }
        let layer = self.images.len() + (layout.extra + index - 1) as usize;
        TextureId(u16::try_from(layer).unwrap_or(0))
    }
}

impl TextureLayout {
    pub const fn still() -> Self {
        Self {
            layout: Layout::Still,
            extra: 0,
            images: 1,
        }
    }

    /// Image shown `ticks` after the start of an animation (the shader does the same).
    pub fn frame_at(&self, ticks: u32) -> u32 {
        match self.layout {
            Layout::Animated { frame_time } if self.images > 1 => {
                (ticks / frame_time.max(1)) % self.images
            }
            _ => 0,
        }
    }

    /// Variant for a block position hash, following the weights of random textures.
    pub fn random_index(&self, hash: u32) -> u32 {
        let Layout::Random { weights } = &self.layout else {
            return 0;
        };
        let total: u32 = weights.iter().sum();
        let mut roll = hash % total.max(1);
        for (index, &weight) in weights.iter().enumerate() {
            if roll < weight {
                return index as u32;
            }
            roll -= weight;
        }
        0
    }
}

/// Stable hash of a block position, for random texture variants.
pub fn position_hash(x: i32, y: i32, z: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^ (h >> 16)
}

/// Reserved name of the fallback texture.
pub fn missing_id() -> Identifier {
    Identifier::builtin("engine", "missing")
}

fn load_palette(packs: &PackSet, errors: &mut Vec<PackError>) -> Palette {
    let mut palette = Palette::default();
    for pack in packs.packs() {
        let path = pack.root.join("palette.ron");
        if path.is_file() {
            match read_ron::<PaletteFile>(&path) {
                Ok(file) => palette.merge(file),
                Err(error) => errors.push(error),
            }
        }
    }
    palette
}

fn layered_sources(
    packs: &PackSet,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, (Loaded, PathBuf)> {
    let mut sources = BTreeMap::new();
    for pack in packs.packs() {
        for (id, path) in category_files(pack, "textures", "ron", errors) {
            match read_ron::<TextureRecipe>(&path) {
                Ok(recipe) => {
                    sources.insert(id, (Loaded::Recipe(recipe), path));
                }
                Err(error) => errors.push(error),
            }
        }
        for (id, path) in category_files(pack, "textures", "png", errors) {
            match read_texture_png(&path) {
                Ok(image) => {
                    let frame_time = match sources.get(&id) {
                        Some((Loaded::Recipe(TextureRecipe::Animated { frame_time, .. }), _)) => {
                            Some(*frame_time)
                        }
                        Some((Loaded::Image(_, frame_time), _)) => *frame_time,
                        _ => None,
                    };
                    sources.insert(id, (Loaded::Image(image, frame_time), path));
                }
                Err(message) => errors.push(PackError::invalid(&path, message)),
            }
        }
        for id in pack.removals.get("textures") {
            if sources.remove(id).is_none() {
                let message = format!("cannot remove `{id}`: no such texture");
                errors.push(PackError::invalid(&pack.root.join("remove.ron"), message));
            }
        }
    }
    sources
}
