use super::RgbaImage;
use super::color::Palette;
use super::recipe::{BASE_SIZE, Painter, TextureRecipe};
use crate::id::Identifier;
use std::collections::BTreeMap;

/// Where a texture comes from once all packs are layered.
pub enum Source<'a> {
    Recipe(&'a TextureRecipe),
    Image(&'a RgbaImage),
}

/// Deeper chains are certainly a reference loop (`a` drawn on `b` drawn on `a`).
const MAX_DEPTH: u32 = 8;

/// Draws texture `id`, following its bases and recolors.
pub fn generate<'a>(
    id: &Identifier,
    lookup: &dyn Fn(&Identifier) -> Option<Source<'a>>,
    palette: &Palette,
) -> Result<RgbaImage, String> {
    generate_inner(id, lookup, palette, &BTreeMap::new(), 0)
}

fn generate_inner<'a>(
    id: &Identifier,
    lookup: &dyn Fn(&Identifier) -> Option<Source<'a>>,
    palette: &Palette,
    swap: &BTreeMap<String, String>,
    depth: u32,
) -> Result<RgbaImage, String> {
    if depth > MAX_DEPTH {
        return Err(format!("texture `{id}` is part of a reference loop"));
    }
    let recipe = match lookup(id).ok_or_else(|| format!("unknown texture `{id}`"))? {
        Source::Image(image) => return Ok(image.clone()),
        Source::Recipe(recipe) => recipe,
    };
    if let TextureRecipe::Recolor { base, swap: inner } = recipe {
        return generate_inner(base, lookup, palette, &compose(inner, swap), depth + 1);
    }
    let base = recipe
        .base()
        .map(|base| generate_inner(base, lookup, palette, swap, depth + 1))
        .transpose()?;
    if let Some(base) = &base
        && (base.width(), base.height()) != (BASE_SIZE, BASE_SIZE)
    {
        return Err(format!(
            "base of `{id}` must be {BASE_SIZE}×{BASE_SIZE} to be painted on"
        ));
    }
    let painter = Painter {
        palette,
        swap,
        seed: seed_of(id),
    };
    recipe.paint(&painter, base)
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
