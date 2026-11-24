//! Storage upgrades on the base pack: layouts, filters, destruction, compression,
//! sorting and the rules of the upgrade slots.

use super::*;
use crate::content::Content;
use crate::inventory::{Container, ItemStack, SlotPolicy};
use crate::pack::PackSet;
use crate::upgrade::SlotOwner;
use std::path::Path;

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    Content::load(&packs, &PackSet::default()).0
}

fn item(content: &Content, name: &str) -> ItemId {
    content.items.key(&name.parse().unwrap()).unwrap()
}

fn stack(content: &Content, name: &str, count: u16) -> ItemStack {
    ItemStack::new(item(content, name), count, &content.items)
}

fn layout(content: &Content, block: &str) -> (usize, usize) {
    let block = content.blocks.key(&block.parse().unwrap()).unwrap();
    let layout = content.slot_layout(block).unwrap();
    (layout.storage, layout.upgrades)
}

#[test]
fn tiers_give_rows_and_upgrade_slots() {
    let content = content();
    assert_eq!(layout(&content, "strata:chest"), (27, 0));
    assert_eq!(layout(&content, "strata:iron_barrel"), (45, 1));
    assert_eq!(layout(&content, "strata:gold_chest"), (63, 2));
    assert_eq!(layout(&content, "strata:diamond_chest"), (81, 3));
    assert_eq!(layout(&content, "strata:furnace"), (3, 0));
    assert_eq!(layout(&content, "strata:diamond_furnace"), (3, 3));
}

#[test]
fn a_filter_allows_or_refuses_what_it_lists_and_an_empty_one_does_nothing() {
    let content = content();
    let (dirt, stone) = (
        item(&content, "strata:dirt"),
        item(&content, "strata:stone"),
    );
    let mut upgrade = stack(&content, "strata:filter_upgrade", 1);
    let empty = filter_of(&upgrade, &content.items);
    assert!(empty.lets_in(dirt) && !empty.selects(dirt));
    assert!(set_filter_cell(
        &mut upgrade,
        0,
        Some(stone),
        &content.items
    ));
    assert!(
        !set_filter_cell(&mut upgrade, 5, Some(stone), &content.items),
        "listed once"
    );
    let allow = filter_of(&upgrade, &content.items);
    assert!(allow.lets_in(stone) && !allow.lets_in(dirt));
    toggle_filter_mode(&mut upgrade, &content.items);
    let deny = filter_of(&upgrade, &content.items);
    assert!(!deny.lets_in(stone) && deny.lets_in(dirt));
    assert!(set_filter_cell(&mut upgrade, 0, None, &content.items));
    assert!(filter_of(&upgrade, &content.items).is_empty());
}

#[test]
fn destruction_takes_only_what_came_in() {
    let content = content();
    let gravel = item(&content, "strata:gravel");
    let mut void = stack(&content, "strata:void_upgrade", 1);
    set_filter_cell(&mut void, 0, Some(gravel), &content.items);
    let rules = Upkeep::new(&content.upgrades, &content.items, &[&void]);
    let mut before = Container::new(4);
    before.slots[0] = Some(stack(&content, "strata:gravel", 20));
    before.slots[2] = Some(stack(&content, "strata:dirt", 5));
    let mut storage = before.clone();
    assert_eq!(
        destroy_entered(&before, &mut storage, &rules),
        0,
        "already there"
    );
    storage.slots.swap(0, 3);
    assert_eq!(
        destroy_entered(&before, &mut storage, &rules),
        0,
        "moved inside"
    );
    storage.slots[3] = Some(stack(&content, "strata:gravel", 30));
    storage.slots[1] = Some(stack(&content, "strata:gravel", 4));
    assert_eq!(
        destroy_entered(&before, &mut storage, &rules),
        14,
        "10 + 4 came in"
    );
    assert_eq!(storage.count(gravel), 20);
    assert_eq!(storage.get(2).map(|s| s.count), Some(5));
}

#[test]
fn two_filters_of_a_double_chest_both_count() {
    let content = content();
    let (stone, dirt) = (
        item(&content, "strata:stone"),
        item(&content, "strata:dirt"),
    );
    let mut first = stack(&content, "strata:filter_upgrade", 1);
    set_filter_cell(&mut first, 0, Some(stone), &content.items);
    let mut second = stack(&content, "strata:filter_upgrade", 1);
    set_filter_cell(&mut second, 0, Some(dirt), &content.items);
    toggle_filter_mode(&mut second, &content.items);
    let rules = Upkeep::new(&content.upgrades, &content.items, &[&first, &second]);
    assert!(rules.lets_in(stone));
    assert!(!rules.lets_in(dirt), "refused by both");
    assert!(
        !rules.lets_in(item(&content, "strata:gravel")),
        "not allowed by the first"
    );
}

#[test]
fn compression_chains_and_waits_for_room() {
    let content = content();
    let compressions = &content.storage.compressions;
    let mut storage = Container::new(3);
    storage.slots[0] = Some(stack(&content, "strata:gold_nugget", 64));
    storage.slots[1] = Some(stack(&content, "strata:gold_nugget", 26));
    assert!(compress(&mut storage, compressions, &content.items));
    let gold = |name| storage.count(item(&content, name));
    assert_eq!(
        (
            gold("strata:gold_nugget"),
            gold("strata:gold_ingot"),
            gold("strata:gold_block")
        ),
        (0, 1, 1),
        "90 nuggets: 10 ingots, then a block and an ingot"
    );
    let mut full = Container::new(1);
    full.slots[0] = Some(stack(&content, "strata:iron_nugget", 8));
    assert!(
        !compress(&mut full, compressions, &content.items),
        "not nine yet"
    );
    let mut worn = Container::new(2);
    let mut odd = stack(&content, "strata:iron_nugget", 9);
    odd.data.set(
        "strata:mark".parse().unwrap(),
        crate::inventory::DataValue::Int(1),
    );
    worn.slots[0] = Some(odd);
    assert!(
        !compress(&mut worn, compressions, &content.items),
        "only plain stacks"
    );
}

#[test]
fn upkeep_sorts_only_when_asked() {
    let content = content();
    let sorting = stack(&content, "strata:sorting_upgrade", 1);
    let rules = Upkeep::new(&content.upgrades, &content.items, &[&sorting]);
    assert!(rules.sort && rules.acts() && rules.magnet.is_none());
    let mut storage = Container::new(5);
    storage.slots[3] = Some(stack(&content, "strata:dirt", 4));
    let before = storage.clone();
    let open = after_entry(&before, &mut storage, &rules, &[], &content.items, false);
    assert!(!open && storage.get(3).is_some(), "not under the pointer");
    assert!(after_entry(
        &before,
        &mut storage,
        &rules,
        &[],
        &content.items,
        true
    ));
    assert_eq!(storage.get(0).map(|s| s.count), Some(4));
}

#[test]
fn upgrade_slots_take_storage_upgrades_once_and_shift_clicks_follow_the_filter() {
    let content = content();
    let family = "strata:storage".parse().unwrap();
    let chest = content
        .blocks
        .key(&"strata:iron_chest".parse().unwrap())
        .unwrap();
    let chest_item = content.items.iter().find_map(|(key, _, i)| {
        matches!(i.kind, crate::content::ItemKind::Block(b) if b == chest).then_some(key)
    });
    let mut filter = stack(&content, "strata:filter_upgrade", 1);
    set_filter_cell(
        &mut filter,
        0,
        Some(item(&content, "strata:stone")),
        &content.items,
    );
    let policy = StorageSlots {
        items: &content.items,
        upgrades: &content.upgrades,
        owner: SlotOwner {
            family: &family,
            item: chest_item,
        },
        storage: 2,
        filters: vec![filter_of(&filter, &content.items)],
    };
    let mut slots = Container::new(4);
    let magnet = stack(&content, "strata:magnet_upgrade", 1);
    assert!(policy.accepts(&slots, 2, &magnet));
    assert!(!policy.accepts(&slots, 2, &stack(&content, "strata:dirt", 1)));
    assert!(!policy.accepts(&slots, 2, &stack(&content, "strata:speed_module", 1)));
    slots.slots[3] = Some(magnet.clone());
    assert!(
        !policy.accepts(&slots, 2, &magnet),
        "the same upgrade twice"
    );
    assert_eq!(
        policy.route(&slots, &stack(&content, "strata:stone", 1)),
        vec![0, 1]
    );
    assert!(
        policy
            .route(&slots, &stack(&content, "strata:dirt", 1))
            .is_empty()
    );
    assert_eq!(policy.storage(), Some(0..2));
}
