//! Builds the wiki: one static site per language (fr, en, lol) in `wiki/`, opened straight
//! from `wiki/index.html`. Data comes from the packs, texts from `wiki-content/`, images
//! from `strata --render-icons` and `--render-biomes`.

mod biomes;
mod blocks;
mod history;
mod html;
mod pages;
mod reference;
mod search;
mod site;
mod texts;

use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "wikigen", about = "Builds the Strata Mundi wiki")]
struct Args {
    /// Fail when an entry has no written text in some language.
    #[arg(long)]
    strict: bool,
    /// Where the site is written (replaced entirely).
    #[arg(long, default_value = "wiki")]
    out: PathBuf,
    /// Game assets (packs).
    #[arg(long, default_value = "assets")]
    assets: PathBuf,
    /// Written texts and `history.ron`.
    #[arg(long, default_value = "wiki-content")]
    content: PathBuf,
    /// Icons and biome views rendered by the game (`icons/`, `biomes/`).
    #[arg(long, default_value = "wiki-images")]
    images: PathBuf,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = Args::parse();
    let data = site::Data::load(&args.assets, &args.content, &args.images)?;
    let report = site::build(&data, &args.out)?;
    for missing in &report.missing {
        tracing::warn!("no text: {missing}");
    }
    tracing::info!(
        pages = report.pages,
        missing = report.missing.len(),
        out = %args.out.display(),
        "wiki written"
    );
    if args.strict && !report.missing.is_empty() {
        anyhow::bail!("{} texts missing (strict mode)", report.missing.len());
    }
    Ok(())
}
