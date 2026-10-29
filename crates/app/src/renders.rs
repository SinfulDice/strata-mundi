//! Images for the wiki: item icons (no GPU needed) and one view of each biome.

use crate::offline;
use anyhow::Context;
use glam::DVec3;
use std::path::Path;
use std::sync::Arc;
use strata_client::camera::Camera;
use strata_client::catalog::item_icon;
use strata_client::screenshot::{View, capture, write_png};
use strata_client::settings::GraphicsSettings;
use strata_core::content::Content;
use strata_core::id::Identifier;
use strata_worldgen::{Generator, Seed, WorldgenContent};

const ICON_SIZE: u32 = 64;
/// Same world for every release, so biome pictures only change with the generation.
const BIOME_SEED: &str = "Strata Mundi wiki";
/// Spacing of the search for a biome, and how far it goes.
const SEARCH_STEP: i32 = 48;
const SEARCH_RINGS: i32 = 160;
/// The camera stands this far back from the place it shows, on both axes.
const BACK: i32 = 72;
/// The view needs this much of the same biome around it, in blocks.
const BIOME_MARGIN: i32 = 40;

/// `<dir>/<namespace>/<name>.png` for every item.
pub fn render_icons(content: &Content, dir: &Path) -> anyhow::Result<()> {
    for (_, id, item) in content.items.iter() {
        let Some(icon) = item_icon(content, item.kind, ICON_SIZE) else {
            continue;
        };
        write_png(&image_path(dir, id), &icon)?;
    }
    tracing::info!(dir = %dir.display(), "item icons written");
    Ok(())
}

/// `<dir>/<namespace>/<name>.png`: each biome seen from a little above, at the first
/// place of the reference world where it is wide enough.
pub fn render_biomes(
    content: &Content,
    worldgen: &Arc<WorldgenContent>,
    graphics: GraphicsSettings,
    dir: &Path,
) -> anyhow::Result<()> {
    let generator = Arc::new(Generator::new(
        Seed::from_text(BIOME_SEED),
        Arc::clone(worldgen),
    ));
    for biome in worldgen.biomes() {
        let Some((x, z)) = find_biome(&generator, &biome.id) else {
            tracing::warn!(biome = %biome.id, "biome not found near the origin");
            continue;
        };
        let ground = |x, z| {
            generator
                .sample(x, z)
                .0
                .height
                .max(worldgen.terrain.sea_level)
        };
        let (ex, ez) = (x - BACK, z - BACK);
        let target = f64::from(ground(x, z));
        // Above everything between the camera and what it looks at, so nothing hides it.
        let highest = (0..=BACK / 4)
            .map(|step| ground(ex + step * 4, ez + step * 4))
            .max()
            .unwrap_or(0);
        let eye_y = (target + 40.0).max(f64::from(highest) + 16.0);
        let eye = DVec3::new(f64::from(ex), eye_y, f64::from(ez));
        let distance = f64::from(BACK) * std::f64::consts::SQRT_2;
        let pitch = ((target - eye_y) / distance).atan().to_degrees() as f32;
        let world = offline::generate_around(&generator, eye.x, eye.z, graphics.render_distance);
        let view = View {
            camera: Camera::new(eye, 135.0, pitch, 70.0),
            size: (960, 540),
            graphics: graphics.clone(),
        };
        let path = image_path(dir, &biome.id);
        capture(content, &world, &view, &path).with_context(|| format!("biome {}", biome.id))?;
    }
    tracing::info!(dir = %dir.display(), "biome views written");
    Ok(())
}

/// Spiral outward from the origin until the biome covers a whole neighbourhood.
fn find_biome(generator: &Generator, id: &Identifier) -> Option<(i32, i32)> {
    let is = |x: i32, z: i32| &generator.sample(x, z).1.id == id;
    let wide = |x: i32, z: i32| {
        [(0, 0), (-1, -1), (1, -1), (-1, 1), (1, 1)]
            .iter()
            .all(|(dx, dz)| is(x + dx * BIOME_MARGIN, z + dz * BIOME_MARGIN))
    };
    (0..SEARCH_RINGS).find_map(|ring| {
        ring_points(ring)
            .map(|(x, z)| (x * SEARCH_STEP, z * SEARCH_STEP))
            .find(|&(x, z)| wide(x, z))
    })
}

/// The points at Chebyshev distance `ring` from the origin.
fn ring_points(ring: i32) -> impl Iterator<Item = (i32, i32)> {
    (-ring..=ring)
        .flat_map(move |x| (-ring..=ring).map(move |z| (x, z)))
        .filter(move |&(x, z)| x.abs().max(z.abs()) == ring)
}

fn image_path(dir: &Path, id: &Identifier) -> std::path::PathBuf {
    dir.join(id.namespace()).join(format!("{}.png", id.name()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_surround_the_origin() {
        assert_eq!(ring_points(0).count(), 1);
        assert_eq!(ring_points(2).count(), 16);
    }
}
