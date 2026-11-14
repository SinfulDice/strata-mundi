#![allow(clippy::unwrap_used)]

use super::tests::{World, block, place, wait, world};
use super::*;
use strata_core::content::ItemId;
use strata_core::inventory::{Area, Click, MAIN, SlotRef, SortOrder};
use strata_protocol::{InventoryRequest, WorldAction};

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn survival() -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    w.game.take_messages();
    w
}

fn request(w: &mut World, request: InventoryRequest) {
    w.game
        .inventory_request(&w.loader, request, &|item, _| item.0.to_string());
}

fn put(w: &mut World, index: usize, name: &str, count: u16) {
    let stack = ItemStack::new(item(w, name), count, &w.game.rules);
    w.game
        .with_inventory(|inventory| inventory.slots.slots[index] = Some(stack));
}

fn count(w: &World, name: &str) -> u32 {
    w.game.inventory().slots.count(item(w, name))
}

fn click(w: &mut World, slot: SlotRef, click: Click) {
    request(w, InventoryRequest::Click { slot, click });
}

#[test]
fn clicks_are_applied_by_the_server_and_answered_once() {
    let mut w = survival();
    put(&mut w, 0, "strata:stone", 10);
    click(&mut w, SlotRef::player(0), Click::Secondary);
    click(&mut w, SlotRef::player(MAIN.start), Click::Primary);
    let inventory = w.game.inventory();
    assert_eq!(inventory.slots.get(0).unwrap().count, 5);
    assert_eq!(inventory.slots.get(MAIN.start).unwrap().count, 5);
    let answers = w.game.take_messages();
    let inventories = answers
        .iter()
        .filter(|m| matches!(m, ServerMessage::Inventory(_)))
        .count();
    assert_eq!(inventories, 1, "one state per batch of messages");
    request(&mut w, InventoryRequest::Select(12));
    assert_eq!(w.game.inventory().selected, 0, "no tenth hotbar slot");
    request(&mut w, InventoryRequest::Select(4));
    assert_eq!(w.game.inventory().selected, 4);
}

#[test]
fn creative_may_put_any_stack_survival_may_not() {
    let mut w = world();
    let stone = ItemStack::new(item(&w, "strata:stone"), 64, &w.game.rules);
    let too_many = ItemStack {
        count: 65,
        ..stone.clone()
    };
    request(
        &mut w,
        InventoryRequest::CreativeSet {
            index: 3,
            stack: Some(stone.clone()),
        },
    );
    request(
        &mut w,
        InventoryRequest::CreativeSet {
            index: 4,
            stack: Some(too_many),
        },
    );
    request(
        &mut w,
        InventoryRequest::CreativeSet {
            index: 36,
            stack: Some(stone.clone()),
        },
    );
    let inventory = w.game.inventory();
    assert_eq!(inventory.slots.get(3), Some(&stone));
    assert!(inventory.slots.get(4).is_none() && inventory.slots.get(36).is_none());
    w.game.set_mode(GameMode::Survival);
    request(
        &mut w,
        InventoryRequest::CreativeSet {
            index: 5,
            stack: Some(stone),
        },
    );
    assert!(w.game.inventory().slots.get(5).is_none());
}

#[test]
fn sorting_by_name_keeps_locked_slots() {
    let mut w = survival();
    put(&mut w, MAIN.start, "strata:stone", 3);
    put(&mut w, MAIN.start + 1, "strata:dirt", 3);
    put(&mut w, MAIN.start + 2, "strata:sand", 3);
    request(&mut w, InventoryRequest::ToggleLock(MAIN.start as u16 + 2));
    let dirt = item(&w, "strata:dirt");
    let names = move |item: ItemId, _: &str| if item == dirt { "a" } else { "b" }.to_owned();
    let sort = InventoryRequest::Sort {
        area: Area::Player,
        order: SortOrder::Name,
        language: "fr".into(),
    };
    w.game.inventory_request(&w.loader, sort, &names);
    let inventory = w.game.inventory();
    assert_eq!(inventory.slots.get(MAIN.start).unwrap().item, dirt);
    assert_eq!(
        inventory.slots.get(MAIN.start + 1).unwrap().item,
        item(&w, "strata:stone")
    );
    assert_eq!(
        inventory.slots.get(MAIN.start + 2).unwrap().item,
        item(&w, "strata:sand")
    );
}

#[test]
fn survival_uses_up_placed_blocks_and_breaking_drops_them() {
    let mut w = survival();
    let planks = block(&w.content, "strata:oak_planks");
    put(&mut w, 0, "strata:oak_planks", 2);
    let against = BlockPos::new(10, 63, 8);
    let id = strata_protocol::ActionId(1);
    let action = WorldAction::Place {
        against,
        face: Face::Up,
        block: planks,
    };
    assert_eq!(w.game.request_action(&mut w.loader, id, action).len(), 1);
    assert_eq!(count(&w, "strata:oak_planks"), 1);
    wait(&mut w, 2);
    let placed = against.neighbour(Face::Up);
    assert!(!w.game.break_block(&mut w.loader, placed).is_empty());
    assert_eq!(w.game.ecs.drops.len(), 1, "the block's item lies there");
    wait(&mut w, 20);
    assert_eq!(w.game.ecs.drops.len(), 1, "too far to be picked up");
    let body = w.game.ecs.bodies.get_mut(w.game.player).unwrap();
    body.position.x = 10.0;
    wait(&mut w, 2);
    assert_eq!(w.game.ecs.drops.len(), 0, "picked up");
    assert_eq!(count(&w, "strata:oak_planks"), 2);
    hold_nothing(&mut w);
    let refused = WorldAction::Place {
        against,
        face: Face::Up,
        block: planks,
    };
    let id = strata_protocol::ActionId(2);
    assert!(
        w.game.request_action(&mut w.loader, id, refused).is_empty(),
        "nothing in hand"
    );
}

fn hold_nothing(w: &mut World) {
    w.game.with_inventory(|inventory| inventory.selected = 5);
}

#[test]
fn thrown_items_wait_merge_and_expire() {
    let mut w = survival();
    put(&mut w, 0, "strata:dirt", 5);
    request(&mut w, InventoryRequest::DropSelected { whole: false });
    request(&mut w, InventoryRequest::DropSelected { whole: false });
    assert_eq!(count(&w, "strata:dirt"), 3);
    assert_eq!(w.game.ecs.drops.len(), 2);
    wait(&mut w, 20);
    assert_eq!(w.game.ecs.drops.len(), 1, "two close equal stacks join");
    assert_eq!(
        count(&w, "strata:dirt"),
        3,
        "a thrown item is not taken back at once"
    );
    let far = DVec3::new(2.5, 64.0, 2.5);
    let stone = ItemStack::new(item(&w, "strata:stone"), 1, &w.game.rules);
    w.game.spawn_drop(stone, far, DVec3::ZERO);
    let lifetime = w.game.rules.dropped_item_entity().unwrap().1.lifetime;
    for (_, drop) in w.game.ecs.drops.iter_mut() {
        drop.age = lifetime * 20 - 2;
    }
    w.game.take_messages();
    wait(&mut w, 3);
    assert_eq!(w.game.ecs.drops.len(), 0, "gone after the lifetime");
    let removed = w.game.take_messages();
    assert!(
        removed
            .iter()
            .any(|m| matches!(m, ServerMessage::EntityRemoved(_)))
    );
}

#[test]
fn a_bucket_is_filled_and_emptied_in_survival() {
    let mut w = survival();
    put(&mut w, 0, "strata:bucket", 2);
    let water = block(&w.content, "strata:water");
    let pool = BlockPos::new(9, 63, 8);
    assert!(w.loader.set_block(pool, water));
    let bucket = item(&w, "strata:bucket");
    assert!(
        !w.game
            .use_item(&mut w.loader, bucket, pool, Face::Up)
            .is_empty()
    );
    assert_eq!(count(&w, "strata:bucket"), 1);
    assert_eq!(count(&w, "strata:water_bucket"), 1);
}

#[test]
fn chests_open_join_in_pairs_and_drop_their_contents() {
    let mut w = survival();
    let chest = block(&w.content, "strata:chest");
    let row = [6, 7, 8].map(|x| BlockPos::new(x, 64, 6));
    for pos in row {
        assert!(w.loader.set_block(pos, chest));
    }
    assert_eq!(w.game.partner(&w.loader, row[0]), Some(row[1]));
    assert_eq!(w.game.partner(&w.loader, row[1]), Some(row[0]));
    assert_eq!(
        w.game.partner(&w.loader, row[2]),
        None,
        "the third stays single"
    );
    assert!(w.game.open_block(&w.loader, row[1]));
    let opened = w.game.take_messages().into_iter().find_map(|m| match m {
        ServerMessage::ContainerOpened(open) => Some(open),
        _ => None,
    });
    assert_eq!(opened.unwrap().contents.len(), 54);
    put(&mut w, 0, "strata:stone", 30);
    click(&mut w, SlotRef::player(0), Click::Primary);
    click(&mut w, SlotRef::open(40), Click::Primary);
    assert_eq!(
        w.game.block_entities[&row[1]]
            .container
            .get(13)
            .unwrap()
            .count,
        30
    );
    wait(&mut w, 2);
    assert!(!w.game.break_block(&mut w.loader, row[1]).is_empty());
    assert!(!w.game.block_entities.contains_key(&row[1]));
    let dropped: u16 = w.game.ecs.drops.iter().map(|(_, d)| d.stack.count).sum();
    assert_eq!(dropped, 31, "the contents and the chest itself");
    assert!(
        w.game
            .take_messages()
            .contains(&ServerMessage::ContainerClosed)
    );
}

#[test]
fn quick_deposit_fills_chests_that_hold_the_item() {
    let mut w = survival();
    let barrel = block(&w.content, "strata:barrel");
    let pos = BlockPos::new(8, 64, 6);
    place_container(&mut w, barrel, pos);
    let stone = ItemStack::new(item(&w, "strata:stone"), 1, &w.game.rules);
    w.game.block_entities.entry(pos).or_default().container = {
        let mut c = strata_core::inventory::Container::new(27);
        c.slots[0] = Some(stone);
        c
    };
    put(&mut w, MAIN.start, "strata:stone", 20);
    put(&mut w, MAIN.start + 1, "strata:dirt", 20);
    request(&mut w, InventoryRequest::QuickDeposit);
    assert_eq!(
        w.game.block_entities[&pos]
            .container
            .count(item(&w, "strata:stone")),
        21
    );
    assert_eq!(count(&w, "strata:stone"), 0);
    assert_eq!(count(&w, "strata:dirt"), 20);
}

fn place_container(w: &mut World, container: BlockId, pos: BlockPos) {
    let changes = place(w, pos.neighbour(Face::Down), Face::Up, container);
    assert_eq!(changes, vec![(pos, container)]);
}
