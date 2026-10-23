use crate::cli::Args;
use crate::paths;
use anyhow::Context;
use std::path::PathBuf;
use strata_client::preview::PreviewScene;
use strata_client::render::MeshData;
use strata_core::content::Content;
use strata_core::pack::PackSet;

pub struct Loaded {
    pub content: Content,
    pub scene: MeshData,
    pub assets: PathBuf,
}

/// Game packs from `assets/packs`, then the resource packs chosen on the command line.
/// Pack problems are logged, never fatal.
pub fn load(args: &Args) -> anyhow::Result<Loaded> {
    let assets = paths::assets_dir().context("assets folder not found")?;
    let mut errors = Vec::new();
    let packs = PackSet::discover(&assets.join("packs"), &mut errors);
    let resource_packs = PackSet::named(&args.resource_packs_dir, &args.resource_pack, &mut errors);
    let (content, more) = Content::load(&packs, &resource_packs);
    errors.extend(more);
    let scene = PreviewScene::load(&assets.join("scenes/preview.ron"), &content)
        .map(|scene| scene.mesh(&content))
        .unwrap_or_else(|error| {
            tracing::warn!(%error, "preview scene unavailable");
            MeshData::default()
        });
    for error in &errors {
        tracing::warn!("{error}");
    }
    tracing::info!(
        blocks = content.blocks.len(),
        items = content.items.len(),
        textures = content.textures.images.len(),
        resolution = content.textures.resolution,
        problems = errors.len(),
        "content loaded"
    );
    Ok(Loaded {
        content,
        scene,
        assets,
    })
}
