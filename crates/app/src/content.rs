use crate::cli::Args;
use crate::paths;
use anyhow::Context;
use std::path::PathBuf;
use std::sync::Arc;
use strata_client::preview::load_preview;
use strata_client::ui::UiAssets;
use strata_client::world::ClientWorld;
use strata_core::content::Content;
use strata_core::dimension::DimensionId;
use strata_core::pack::{PackError, PackSet};
use strata_worldgen::WorldgenContent;

pub struct Loaded {
    pub content: Content,
    pub ui: UiAssets,
    pub worldgen: Arc<WorldgenContent>,
    pub assets: PathBuf,
}

/// Game packs from `assets/packs`, then the resource packs chosen on the command line.
/// Pack problems are logged, never fatal; only a missing dimension stops the game.
pub fn load(args: &Args, dimension: &DimensionId) -> anyhow::Result<Loaded> {
    let assets = paths::assets_dir().context("assets folder not found")?;
    let mut errors = Vec::new();
    let packs = PackSet::discover(&assets.join("packs"), &mut errors);
    let resource_packs = PackSet::named(&args.resource_packs_dir, &args.resource_pack, &mut errors);
    let (content, more) = Content::load(&packs, &resource_packs);
    errors.extend(more);
    let worldgen = WorldgenContent::load(&packs, &content.blocks, &dimension.0, &mut errors);
    let presentation = PackSet::joined(&packs, &resource_packs);
    let ui = UiAssets::load(&assets.join("ui"), &presentation, &mut errors);
    report(&errors);
    let ui = ui.context("interface files are missing or broken")?;
    let worldgen = worldgen.with_context(|| format!("dimension `{dimension}` is not usable"))?;
    tracing::info!(
        blocks = content.blocks.len(),
        textures = content.textures.images.len(),
        resolution = content.textures.resolution,
        biomes = worldgen.biomes().len(),
        problems = errors.len(),
        "content loaded"
    );
    Ok(Loaded {
        content,
        ui,
        worldgen: Arc::new(worldgen),
        assets,
    })
}

pub fn preview(loaded: &Loaded) -> ClientWorld {
    load_preview(&loaded.assets.join("scenes/preview.ron"), &loaded.content).unwrap_or_else(
        |error| {
            tracing::warn!(%error, "preview scene unavailable");
            ClientWorld::default()
        },
    )
}

fn report(errors: &[PackError]) {
    for error in errors {
        tracing::warn!("{error}");
    }
}
