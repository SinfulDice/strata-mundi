use super::png_file::{missing_texture, read_texture_png, upscale};
use super::registry::{Registry, TextureId};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, category_files, read_ron};
use crate::texture::{BASE_SIZE, Palette, PaletteFile, RgbaImage, Source, TextureRecipe, generate};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Every texture of the game at one common resolution, ready for the GPU texture array.
#[derive(Debug, Clone)]
pub struct TextureLibrary {
    pub images: Registry<TextureId, RgbaImage>,
    pub resolution: u32,
}

enum Loaded {
    Recipe(TextureRecipe),
    Image(RgbaImage),
}

impl TextureLibrary {
    /// `textures/` of every pack, PNG over RON within a pack, later packs on top.
    pub fn load(packs: &PackSet, errors: &mut Vec<PackError>) -> Self {
        let palette = load_palette(packs, errors);
        let sources = layered_sources(packs, errors);
        let resolution = sources
            .values()
            .filter_map(|(loaded, _)| match loaded {
                Loaded::Image(image) => Some(image.width()),
                Loaded::Recipe(_) => None,
            })
            .fold(BASE_SIZE, u32::max);
        let lookup = |id: &Identifier| {
            sources.get(id).map(|(loaded, _)| match loaded {
                Loaded::Recipe(recipe) => Source::Recipe(recipe),
                Loaded::Image(image) => Source::Image(image),
            })
        };
        let mut images = Registry::default();
        images.insert(missing_id(), missing_texture(resolution));
        for (id, (_, path)) in &sources {
            let image = generate(id, &lookup, &palette).unwrap_or_else(|message| {
                errors.push(PackError::invalid(path, message));
                missing_texture(BASE_SIZE)
            });
            images.insert(id.clone(), upscale(&image, resolution));
        }
        Self { images, resolution }
    }

    /// Layer of `id`, or the "missing" layer.
    pub fn layer(&self, id: &Identifier) -> Option<TextureId> {
        self.images.key(id)
    }
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
                    sources.insert(id, (Loaded::Image(image), path));
                }
                Err(message) => errors.push(PackError::invalid(&path, message)),
            }
        }
        for id in pack.removals.get("textures") {
            sources.remove(id);
        }
    }
    sources
}
