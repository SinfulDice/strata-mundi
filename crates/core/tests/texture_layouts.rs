#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use strata_core::content::{Content, TextureId, TextureLayout, position_hash};
use strata_core::id::Identifier;
use strata_core::pack::PackSet;
use strata_core::texture::{Connection, Layout, Rgba};

fn game() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let mut errors = Vec::new();
    let packs = PackSet::discover(&dir, &mut errors);
    let (content, more) = Content::load(&packs, &PackSet::default());
    errors.extend(more);
    assert!(errors.is_empty(), "{errors:#?}");
    content
}

fn fixtures(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn texture(content: &Content, name: &str) -> TextureId {
    content
        .textures
        .layer(&name.parse::<Identifier>().unwrap())
        .unwrap()
}

#[test]
fn animations_show_their_frames_in_turn() {
    let content = game();
    for (name, frames) in [
        ("strata:water", 8),
        ("strata:lava", 8),
        ("strata:fire", 8),
        ("strata:portal", 16),
    ] {
        let layout = content.textures.layout(texture(&content, name));
        assert!(matches!(layout.layout, Layout::Animated { .. }), "{name}");
        assert_eq!(layout.images, frames, "{name}");
    }
    let fire = content.textures.layout(texture(&content, "strata:fire"));
    let Layout::Animated { frame_time } = fire.layout else {
        unreachable!()
    };
    assert_eq!(fire.frame_at(0), 0);
    assert_eq!(fire.frame_at(frame_time - 1), 0);
    assert_eq!(fire.frame_at(frame_time), 1);
    assert_eq!(fire.frame_at(frame_time * 8), 0, "the animation loops");
    assert_eq!(TextureLayout::still().frame_at(1234), 0);
}

#[test]
fn extra_images_follow_every_first_image() {
    let content = game();
    let library = &content.textures;
    let water = texture(&content, "strata:water");
    assert_eq!(library.image_layer(water, 0), water);
    let frames: Vec<_> = (1..8).map(|i| library.image_layer(water, i).0).collect();
    assert!(
        frames
            .iter()
            .all(|&layer| usize::from(layer) >= library.images.len())
    );
    assert!(frames.windows(2).all(|pair| pair[1] == pair[0] + 1));
    assert_eq!(
        library.image_layer(water, 8),
        water,
        "out of range: first image"
    );
    assert_eq!(library.layers().count(), library.layer_count());
    let distinct = (0..8)
        .map(|i| {
            library
                .layers()
                .nth(usize::from(library.image_layer(water, i).0))
        })
        .collect::<Vec<_>>();
    assert_ne!(distinct[0], distinct[1]);
}

#[test]
fn the_game_fits_the_smallest_gpu_texture_array() {
    let content = game();
    assert!(
        content.textures.layer_count() <= 256,
        "{} layers; every Vulkan device has 256",
        content.textures.layer_count()
    );
}

#[test]
fn png_strips_replace_animations_and_keep_their_pace() {
    let mut errors = Vec::new();
    let packs = PackSet::discover(&fixtures("demo"), &mut errors);
    let resources = PackSet::named(&fixtures("resource"), &["restyle".into()], &mut errors);
    let (content, more) = Content::load(&packs, &resources);
    errors.extend(more);
    assert!(errors.is_empty(), "{errors:#?}");
    let glow = texture(&content, "base:glow");
    let layout = content.textures.layout(glow);
    assert_eq!(layout.layout, Layout::Animated { frame_time: 5 });
    assert_eq!(layout.images, 3);
    let third = content.textures.image_layer(glow, 2);
    let image = content.textures.layers().nth(usize::from(third.0)).unwrap();
    assert_eq!(image.get(0, 0), Rgba([0, 0, 255, 255]));
}

#[test]
fn random_variants_follow_their_weights() {
    let layout = TextureLayout {
        layout: Layout::Random {
            weights: vec![3, 1],
        },
        extra: 0,
        images: 2,
    };
    let picks: Vec<u32> = (0..4).map(|hash| layout.random_index(hash)).collect();
    assert_eq!(picks, [0, 0, 0, 1]);
    assert_eq!(position_hash(5, 70, -3), position_hash(5, 70, -3));
    assert_ne!(position_hash(5, 70, -3), position_hash(5, 70, -4));
}

#[test]
fn glass_and_bookshelves_are_connected() {
    let content = game();
    for (name, connect, images) in [
        ("strata:glass", Connection::Edges, 16),
        ("strata:bookshelf", Connection::Vertical, 4),
    ] {
        let layout = content.textures.layout(texture(&content, name));
        assert_eq!(layout.layout, Layout::Connected(connect), "{name}");
        assert_eq!(layout.images, images, "{name}");
    }
}

#[test]
fn natural_blocks_have_random_variants() {
    let content = game();
    for name in [
        "strata:stone",
        "strata:grass_block_top",
        "strata:sand",
        "strata:dirt",
    ] {
        let layout = content.textures.layout(texture(&content, name));
        assert!(matches!(layout.layout, Layout::Random { .. }), "{name}");
        assert_eq!(layout.images, 3, "{name}");
        let images: Vec<_> = (0..3)
            .map(|i| {
                content.textures.layers().nth(usize::from(
                    content.textures.image_layer(texture(&content, name), i).0,
                ))
            })
            .collect();
        assert!(
            images[0] != images[1] && images[1] != images[2],
            "{name}: distinct drawings"
        );
    }
}
