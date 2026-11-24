//! Bags on the base pack.

use super::*;
use crate::content::Content;
use crate::pack::PackSet;
use std::path::Path;

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let (content, errors) = Content::load(&packs, &PackSet::default());
    assert!(errors.is_empty(), "{errors:?}");
    content
}

fn item(content: &Content, name: &str) -> ItemId {
    content.items.key(&name.parse().unwrap()).unwrap()
}

#[test]
fn backpacks_take_the_layout_of_their_block() {
    let content = content();
    let leather = content
        .bags
        .get(item(&content, "strata:leather_backpack"))
        .unwrap();
    assert_eq!((leather.layout.storage, leather.layout.upgrades), (27, 1));
    let diamond = content
        .bags
        .get(item(&content, "strata:diamond_backpack"))
        .unwrap();
    assert_eq!((diamond.layout.storage, diamond.layout.upgrades), (81, 4));
    let pouch = content
        .bags
        .get(item(&content, "strata:seed_pouch"))
        .unwrap();
    assert_eq!((pouch.layout.storage, pouch.layout.upgrades), (9, 0));
}
