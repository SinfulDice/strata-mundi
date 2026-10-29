//! What the wiki is built from, and the build itself: every page of every language,
//! the shared files (style, script, images) and the search indexes.

use crate::history::History;
use crate::texts::Texts;
use crate::{biomes, blocks, html, pages, reference, search};
use anyhow::Context;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use strata_core::content::Content;
use strata_core::id::Identifier;
use strata_core::lang::LANGUAGES;
use strata_core::pack::PackSet;
use strata_worldgen::WorldgenContent;

const DIMENSION: &str = "strata:terra";
/// Left in the output folder, so a later build knows it may replace it.
const MARKER: &str = ".wikigen";

pub struct Data {
    pub content: Content,
    pub worldgen: WorldgenContent,
    pub history: History,
    pub texts: Texts,
    pub images: PathBuf,
}

impl Data {
    pub fn load(assets: &Path, content_dir: &Path, images: &Path) -> anyhow::Result<Self> {
        let mut errors = Vec::new();
        let packs = PackSet::discover(&assets.join("packs"), &mut errors);
        let (content, more) = Content::load(&packs, &PackSet::default());
        errors.extend(more);
        let dimension: Identifier = DIMENSION.parse()?;
        let worldgen = WorldgenContent::load(&packs, &content.blocks, &dimension, &mut errors)
            .context("the overworld dimension is not usable")?;
        for error in &errors {
            tracing::warn!("{error}");
        }
        let texts = Texts::new(content_dir);
        let history = History::load(&texts.history_path())?;
        Ok(Self {
            content,
            worldgen,
            history,
            texts,
            images: images.to_owned(),
        })
    }
}

/// One page, its path relative to the language folder (`blocks/strata/stone.html`).
pub struct Page {
    pub path: String,
    pub title: String,
    pub body: String,
}

/// What pages of one language are made with.
pub struct Ctx<'a> {
    pub data: &'a Data,
    pub lang: &'static str,
    missing: RefCell<Vec<String>>,
}

impl Ctx<'_> {
    /// A missing translation falls back to its key: counted for strict mode.
    pub fn tr(&self, key: &str) -> String {
        let text = self.data.content.translations.text(self.lang, key);
        if text == key {
            self.missing
                .borrow_mut()
                .push(format!("{}.ftl: {key}", self.lang));
        }
        text
    }

    pub fn tr_with(&self, key: &str, args: &[(&str, String)]) -> String {
        self.data
            .content
            .translations
            .text_with(self.lang, key, args)
    }

    /// The written text `key` (`blocks/strata/stone`); a notice when missing (counted
    /// for strict mode).
    pub fn text(&self, key: &str) -> String {
        self.data.texts.html(self.lang, key).unwrap_or_else(|| {
            self.missing
                .borrow_mut()
                .push(format!("{}/{key}", self.lang));
            format!("<p class=\"missing\">{}</p>", self.tr("wiki-text-missing"))
        })
    }

    /// Every page needs its entry in `history.ron`.
    pub fn require_history(&self, key: &str) {
        self.added_in(key);
    }

    /// The version that added `key` ("?" and counted as missing when unknown).
    pub fn added_in(&self, key: &str) -> String {
        match self.data.history.added_in(key) {
            Some(version) => version.to_owned(),
            None => {
                self.missing
                    .borrow_mut()
                    .push(format!("history.ron: {key}"));
                "?".to_owned()
            }
        }
    }
}

pub struct Report {
    pub pages: usize,
    pub missing: Vec<String>,
}

pub fn build(data: &Data, out: &Path) -> anyhow::Result<Report> {
    clean(out)?;
    write_shared(data, out)?;
    write(&out.join(MARKER), "")?;
    let mut report = Report {
        pages: 0,
        missing: Vec::new(),
    };
    for lang in LANGUAGES {
        let ctx = Ctx {
            data,
            lang,
            missing: RefCell::new(Vec::new()),
        };
        let pages = all_pages(&ctx);
        for page in &pages {
            write(&out.join(lang).join(&page.path), &html::layout(&ctx, page))?;
        }
        write(
            &out.join(lang).join("search-index.js"),
            &search::index(&pages),
        )?;
        report.pages += pages.len();
        report.missing.extend(ctx.missing.into_inner());
    }
    report.missing.sort();
    report.missing.dedup();
    Ok(report)
}

fn all_pages(ctx: &Ctx<'_>) -> Vec<Page> {
    let mut pages = vec![pages::home(ctx), pages::news(ctx), pages::changelog(ctx)];
    pages.extend(blocks::pages(ctx));
    pages.extend(biomes::pages(ctx));
    pages.extend(reference::pages(ctx));
    pages
}

/// Style, script, images and the language chooser at the root.
fn write_shared(data: &Data, out: &Path) -> anyhow::Result<()> {
    write(&out.join("style.css"), include_str!("../assets/style.css"))?;
    write(&out.join("wiki.js"), include_str!("../assets/wiki.js"))?;
    write(
        &out.join("index.html"),
        include_str!("../assets/index.html"),
    )?;
    for kind in ["icons", "biomes"] {
        let from = data.images.join(kind);
        if from.is_dir() {
            copy_dir(&from, &out.join("images").join(kind))?;
        } else {
            tracing::warn!(dir = %from.display(), "images not rendered yet");
        }
    }
    Ok(())
}

fn write(path: &Path, text: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| parent.display().to_string())?;
    }
    std::fs::write(path, text).with_context(|| path.display().to_string())
}

/// Empties the output folder, but only one wikigen wrote (or an empty one): a mistyped
/// `--out` must never wipe something else.
fn clean(out: &Path) -> anyhow::Result<()> {
    let Ok(mut entries) = std::fs::read_dir(out) else {
        return Ok(());
    };
    if entries.next().is_some() && !out.join(MARKER).is_file() {
        anyhow::bail!(
            "{} is not a wiki written by wikigen: not replaced",
            out.display()
        );
    }
    std::fs::remove_dir_all(out).with_context(|| format!("cleaning {}", out.display()))
}

fn copy_dir(from: &Path, to: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// `strata:stone` → `strata/stone`, the key of its texts and images.
pub fn id_path(id: &Identifier) -> String {
    format!("{}/{}", id.namespace(), id.name())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The real packs and texts: every entry has its text in every language (strict mode).
    #[test]
    fn every_entry_of_the_game_has_its_texts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = std::env::temp_dir().join(format!("strata-wiki-{}", std::process::id()));
        let data = Data::load(
            &root.join("assets"),
            &root.join("wiki-content"),
            &out.join("none"),
        )
        .unwrap();
        let report = build(&data, &out).unwrap();
        assert!(report.missing.is_empty(), "{:#?}", report.missing);
        assert!(out.join("fr/blocks/strata/stone.html").is_file());
        assert!(out.join("lol/search-index.js").is_file());
        std::fs::remove_dir_all(out).unwrap();
    }
}
