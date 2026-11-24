//! Bags on the base pack.

use super::*;
use crate::content::Content;
use crate::inventory::ItemStack;
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

fn stack(content: &Content, name: &str, count: u16) -> ItemStack {
    ItemStack::new(item(content, name), count, &content.items)
}

#[test]
fn a_shift_click_wears_a_backpack_on_the_back() {
    use crate::inventory::{BACK, Click, PlayerInventory, SlotRef, Window};
    let content = content();
    let mut player = PlayerInventory::default();
    player.slots.slots[3] = Some(stack(&content, "strata:leather_backpack", 1));
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: None,
        policy: None,
    };
    window.click(SlotRef::player(3), Click::Transfer, &content.items);
    assert!(player.slots.get(BACK).is_some());
}

#[test]
fn contents_survive_writing_and_reading() {
    let content = content();
    let rules = content
        .bags
        .get(item(&content, "strata:leather_backpack"))
        .unwrap();
    let mut bag = stack(&content, "strata:leather_backpack", 1);
    let (mut held, cut) = BagContents::read(&bag.data, &rules.layout);
    assert!(cut.is_empty() && held.storage.is_empty());
    held.storage.slots[26] = Some(stack(&content, "strata:dirt", 5));
    held.write(&mut bag.data);
    let (back, _) = BagContents::read(&bag.data, &rules.layout);
    assert_eq!(back, held);
    BagContents::strip(&mut bag.data);
    assert!(bag.data.is_empty(), "an empty bag leaves no trace");
}

#[test]
fn a_pouch_takes_only_its_tag_and_comes_before_the_inventory() {
    use crate::inventory::Container;
    let content = content();
    let world = BagWorld {
        bags: &content.bags,
        upgrades: &content.upgrades,
        items: &content.items,
        compressions: &content.storage.compressions,
    };
    let mut slots = Container::new(crate::inventory::PLAYER_SLOTS);
    slots.slots[2] = Some(stack(&content, "strata:ore_pouch", 1));
    let left = pick_into_bags(&mut slots, stack(&content, "strata:coal", 10), &world);
    assert!(left.is_none());
    let left = pick_into_bags(&mut slots, stack(&content, "strata:dirt", 3), &world);
    assert_eq!(left.map(|s| s.count), Some(3), "dirt is no ore");
    let rules = content
        .bags
        .get(item(&content, "strata:ore_pouch"))
        .unwrap();
    let pouch = slots.get(2).unwrap();
    let held = BagContents::read(&pouch.data, &rules.layout).0;
    assert_eq!(held.storage.count(item(&content, "strata:coal")), 10);
}

#[test]
fn a_tank_holds_one_liquid_up_to_its_capacity() {
    let content = content();
    let mut held = BagContents::default();
    let mut cursor = Some(stack(&content, "strata:water_bucket", 1));
    assert_eq!(
        use_tank(&mut held, 1, &mut cursor, &content.items),
        TankUse::Poured
    );
    let mut water = Some(stack(&content, "strata:water_bucket", 1));
    assert_eq!(
        use_tank(&mut held, 1, &mut water, &content.items),
        TankUse::Nothing,
        "full"
    );
    let mut empties = Some(stack(&content, "strata:bucket", 3));
    let used = use_tank(&mut held, 1, &mut empties, &content.items);
    assert!(
        matches!(used, TankUse::Filled(Some(_))),
        "two empty buckets left on the cursor"
    );
    assert_eq!(empties.map(|s| s.count), Some(2));
    assert_eq!(held.tank, None);
}
