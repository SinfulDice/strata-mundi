#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use strata_core::content::{BlockId, Content, ItemKind, RenderKind, TextureId};
use strata_core::id::Identifier;
use strata_core::lang::LANGUAGES;
use strata_core::pack::{PackError, PackSet};

fn fixtures(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn id(text: &str) -> Identifier {
    text.parse().unwrap()
}

fn load(dir: &Path) -> (Content, Vec<PackError>) {
    let mut errors = Vec::new();
    let packs = PackSet::discover(dir, &mut errors);
    let (content, more) = Content::load(&packs, &PackSet::default());
    errors.extend(more);
    (content, errors)
}

#[test]
fn higher_priority_pack_adds_modifies_and_removes() {
    let (content, errors) = load(&fixtures("demo"));
    assert!(errors.is_empty(), "{errors:#?}");
    let blocks = &content.blocks;
    assert_eq!(blocks.key(&id("strata:air")), Some(BlockId::AIR));
    assert!(blocks.by_identifier(&id("demo:new")).is_some());
    assert_eq!(blocks.by_identifier(&id("base:a")).unwrap().hardness, 9.0);
    assert_eq!(blocks.by_identifier(&id("base:b")).unwrap().hardness, 2.0);
    assert!(blocks.by_identifier(&id("base:c")).is_none());
    let item = content.items.by_identifier(&id("base:b")).unwrap();
    assert!(matches!(item.kind, ItemKind::Block(_)));
}

#[test]
fn broken_pack_gives_clear_errors_without_crashing() {
    let (content, errors) = load(&fixtures("broken"));
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    let mentions = |needle: &str| messages.iter().any(|m| m.contains(needle));
    assert!(mentions("no_manifest: missing pack.ron"), "{messages:#?}");
    assert!(mentions("invalid namespace `Not Valid`"), "{messages:#?}");
    assert!(mentions("typo.ron:3:"), "{messages:#?}");
    assert!(mentions("hardnes"), "{messages:#?}");
    assert!(mentions("unknown texture `good:nothing`"), "{messages:#?}");
    assert!(
        mentions("Bad_Name.ron: invalid identifier"),
        "{messages:#?}"
    );
    assert!(mentions("reference loop"), "{messages:#?}");
    assert!(
        mentions("`octaves` is 99, the maximum is 6"),
        "{messages:#?}"
    );
    let fine = content.blocks.by_identifier(&id("good:fine")).unwrap();
    assert_ne!(fine.faces[0], TextureId::MISSING);
    let missing = content
        .blocks
        .by_identifier(&id("good:no_texture"))
        .unwrap();
    assert_eq!(missing.faces[0], TextureId::MISSING);
    assert!(content.blocks.by_identifier(&id("good:typo")).is_none());
}

fn game_packs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs")
}

#[test]
fn base_game_pack_loads_cleanly() {
    let (content, errors) = load(&game_packs());
    assert!(errors.is_empty(), "{errors:#?}");
    let leaves = content
        .blocks
        .by_identifier(&id("strata:oak_leaves"))
        .unwrap();
    assert_eq!(leaves.render, RenderKind::Cutout);
    assert!(content.textures.images.len() > 20);
}

#[test]
fn every_text_is_translated_in_every_language() {
    let (content, _) = load(&game_packs());
    let missing = content.translations.missing(&content.translation_keys());
    assert!(missing.is_empty(), "missing translations: {missing:#?}");
    for language in LANGUAGES {
        assert_eq!(
            content.translations.text(language, "game-title"),
            "Strata Mundi"
        );
    }
    assert_eq!(
        content.translations.text("fr", "block-strata-stone"),
        "Pierre"
    );
    assert_eq!(
        content.translations.text("lol", "block-strata-stone"),
        "ROCK"
    );
}

fn texture(content: &Content, name: &str) -> strata_core::texture::RgbaImage {
    content
        .textures
        .images
        .by_identifier(&id(name))
        .unwrap()
        .clone()
}

#[test]
fn ores_are_painted_on_high_resolution_png_bases() {
    let (content, errors) = load(&fixtures("hires"));
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(content.textures.resolution, 32);
    let ore = texture(&content, "base:ore");
    assert_eq!((ore.width(), ore.height()), (32, 32));
    assert_ne!(ore, texture(&content, "base:stone"));
}

#[test]
fn generated_textures_are_deterministic() {
    let (first, _) = load(&game_packs());
    let (second, _) = load(&game_packs());
    for (texture_id, name, image) in first.textures.images.iter() {
        assert_eq!(
            second.textures.images.get(texture_id),
            Some(image),
            "{name}"
        );
    }
}

#[test]
fn recolors_chain_their_swaps() {
    let (content, _) = load(&fixtures("demo"));
    let blue = strata_core::texture::Rgba([0, 0, 0x80, 255]);
    assert_eq!(texture(&content, "base:tool_blue").get(3, 3), blue);
}

#[test]
fn resource_packs_restyle_palettes_and_replace_textures() {
    let mut errors = Vec::new();
    let packs = PackSet::discover(&fixtures("demo"), &mut errors);
    let resources = PackSet::named(&fixtures("resource"), &["restyle".into()], &mut errors);
    let (plain, _) = Content::load(&packs, &PackSet::default());
    let (styled, more) = Content::load(&packs, &resources);
    errors.extend(more);
    assert!(errors.is_empty(), "{errors:#?}");
    assert_ne!(texture(&plain, "base:rock"), texture(&styled, "base:rock"));
    let red = strata_core::texture::Rgba([255, 0, 0, 255]);
    assert_eq!(texture(&styled, "base:tool").get(0, 0), red);
}

#[test]
fn missing_translations_are_reported() {
    let (content, _) = load(&fixtures("lang"));
    let missing = content.translations.missing(&[]);
    for expected in [("en", "bye"), ("lol", "hello"), ("lol", "bye")] {
        assert!(
            missing.contains(&(expected.0, expected.1.to_string())),
            "{expected:?} not in {missing:?}"
        );
    }
    assert_eq!(missing.len(), 3);
}
