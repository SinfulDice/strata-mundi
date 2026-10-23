use super::noise::{hash, tileable_fractal};
use super::{Rgba, RgbaImage};

/// How to paint a procedural material: a dark-to-light palette and a noise grain.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialRecipe {
    pub palette: Vec<Rgba>,
    /// Noise lattice cells across the texture: low = large blotches, high = fine grain.
    pub cells: u32,
    pub octaves: u32,
}

/// Same recipe + seed + size always gives the same image.
pub fn generate_material(recipe: &MaterialRecipe, seed: u64, size: u32) -> RgbaImage {
    let mut image = RgbaImage::new(size, size, Rgba::rgb(255, 0, 255));
    let Some(last) = recipe.palette.len().checked_sub(1) else {
        return image;
    };
    for y in 0..size {
        for x in 0..size {
            let grain = tileable_fractal(seed, x, y, size, recipe.cells, recipe.octaves);
            let dither = (hash(seed ^ 0xD17, x as i32, y as i32) % 100) as f32 / 100.0 - 0.5;
            let level = (grain * (last + 1) as f32 + dither * 0.6).clamp(0.0, last as f32);
            image.set(x, y, recipe.palette[level as usize]);
        }
    }
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stone() -> MaterialRecipe {
        MaterialRecipe {
            palette: vec![
                Rgba::rgb(90, 90, 90),
                Rgba::rgb(120, 120, 120),
                Rgba::rgb(150, 150, 150),
            ],
            cells: 4,
            octaves: 2,
        }
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(
            generate_material(&stone(), 1, 16),
            generate_material(&stone(), 1, 16)
        );
        assert_ne!(
            generate_material(&stone(), 1, 16),
            generate_material(&stone(), 2, 16)
        );
    }

    #[test]
    fn only_palette_colors_are_used() {
        let recipe = stone();
        let image = generate_material(&recipe, 5, 16);
        for (x, y) in (0..16).flat_map(|x| (0..16).map(move |y| (x, y))) {
            assert!(recipe.palette.contains(&image.get(x, y)));
        }
    }
}
