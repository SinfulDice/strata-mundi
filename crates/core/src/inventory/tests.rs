#![allow(clippy::unwrap_used)]

use super::*;
use crate::content::{EquipSlot, ItemId};
use crate::id::Identifier;

/// Item 0 stacks to 64, item 1 to 16, item 2 is a tool (1, 50 uses), item 3 a helmet.
struct Rules;

const DIRT: ItemId = ItemId(0);
const PEARL: ItemId = ItemId(1);
const PICK: ItemId = ItemId(2);
const HELMET: ItemId = ItemId(3);

impl ItemRules for Rules {
    fn max_stack(&self, item: ItemId) -> u16 {
        match item.0 {
            0 => 64,
            1 => 16,
            2 | 3 => 1,
            _ => 0,
        }
    }

    fn durability(&self, item: ItemId) -> Option<u16> {
        (item == PICK).then_some(50)
    }

    fn equip(&self, item: ItemId) -> Option<EquipSlot> {
        (item == HELMET).then_some(EquipSlot::Head)
    }
}

fn stack(item: ItemId, count: u16) -> ItemStack {
    ItemStack::new(item, count, &Rules)
}

fn with(slots: &[(usize, ItemStack)]) -> PlayerInventory {
    let mut player = PlayerInventory::default();
    for (index, stack) in slots {
        player.slots.slots[*index] = Some(stack.clone());
    }
    player
}

fn count(slot: Option<&ItemStack>) -> u16 {
    slot.map_or(0, |s| s.count)
}

#[test]
fn stacks_merge_up_to_their_limit() {
    let mut a = stack(PEARL, 10);
    let mut b = stack(PEARL, 10);
    assert_eq!(a.absorb(&mut b, &Rules), 6);
    assert_eq!((a.count, b.count), (16, 4));
    let mut dirt = stack(DIRT, 3);
    assert_eq!(
        a.absorb(&mut dirt, &Rules),
        0,
        "different items never merge"
    );
    assert!(a.is_valid(&Rules) && !stack(PEARL, 17).is_valid(&Rules));
    assert!(
        !stack(ItemId(99), 1).is_valid(&Rules),
        "unknown items are refused"
    );
}

#[test]
fn wear_and_data_keep_stacks_apart() {
    let mut worn = stack(PICK, 1);
    assert_eq!(worn.durability, Some(50));
    assert!(!worn.wear(10));
    assert!(!worn.stacks_with(&stack(PICK, 1)));
    let mut upgraded = stack(DIRT, 1);
    let key: Identifier = "strata:upgrade".parse().unwrap();
    upgraded
        .data
        .set(key.clone(), DataValue::List(vec![DataValue::Int(2)]));
    assert!(!upgraded.stacks_with(&stack(DIRT, 1)));
    assert_eq!(
        upgraded.data.get(&key),
        Some(&DataValue::List(vec![DataValue::Int(2)]))
    );
    assert!(worn.wear(40), "breaks at zero");
}

#[test]
fn splitting_takes_the_larger_half() {
    let mut odd = stack(DIRT, 7);
    assert_eq!(odd.take_half().unwrap().count, 4);
    assert_eq!(odd.count, 3);
    let mut one = stack(DIRT, 1);
    assert_eq!(one.take_half().unwrap().count, 1);
    assert_eq!(one.take(5), None, "nothing left to take");
}

#[test]
fn inserting_joins_then_fills_and_returns_the_rest() {
    let mut chest = Container::new(2);
    chest.slots[1] = Some(stack(PEARL, 12));
    let left = chest.insert_anywhere(stack(PEARL, 30), &Rules).unwrap();
    assert_eq!(
        (count(chest.get(1)), count(chest.get(0)), left.count),
        (16, 16, 10)
    );
    assert_eq!(chest.count(PEARL), 32);
    assert_eq!(chest.remove(PEARL, 20), 20);
    assert_eq!(chest.count(PEARL), 12);
}

#[test]
fn primary_click_takes_puts_merges_and_swaps() {
    let mut player = with(&[
        (0, stack(DIRT, 40)),
        (1, stack(DIRT, 30)),
        (2, stack(PEARL, 3)),
    ]);
    let mut window = Window {
        player: &mut player,
        open: None,
    };
    window.click(SlotRef::player(0), Click::Primary, &Rules);
    assert_eq!(count(window.player.cursor.as_ref()), 40);
    window.click(SlotRef::player(1), Click::Primary, &Rules);
    assert_eq!(count(window.player.slots.get(1)), 64, "merged to the limit");
    assert_eq!(count(window.player.cursor.as_ref()), 6);
    window.click(SlotRef::player(2), Click::Primary, &Rules);
    assert_eq!(window.player.slots.get(2).unwrap().item, DIRT, "exchanged");
    assert_eq!(window.player.cursor.as_ref().unwrap().item, PEARL);
    window.click(SlotRef::player(5), Click::Primary, &Rules);
    assert!(window.player.cursor.is_none());
    assert_eq!(count(window.player.slots.get(5)), 3);
}

#[test]
fn secondary_click_takes_half_and_puts_one() {
    let mut player = with(&[(9, stack(DIRT, 9))]);
    let mut window = Window {
        player: &mut player,
        open: None,
    };
    window.click(SlotRef::player(9), Click::Secondary, &Rules);
    assert_eq!(count(window.player.cursor.as_ref()), 5);
    assert_eq!(count(window.player.slots.get(9)), 4);
    window.click(SlotRef::player(10), Click::Secondary, &Rules);
    window.click(SlotRef::player(10), Click::Secondary, &Rules);
    window.click(SlotRef::player(9), Click::Secondary, &Rules);
    assert_eq!(count(window.player.slots.get(10)), 2);
    assert_eq!(count(window.player.slots.get(9)), 5);
    assert_eq!(count(window.player.cursor.as_ref()), 2);
}

#[test]
fn armour_slots_take_only_their_piece() {
    let mut player = with(&[(0, stack(DIRT, 1)), (1, stack(HELMET, 1))]);
    let mut window = Window {
        player: &mut player,
        open: None,
    };
    window.click(SlotRef::player(0), Click::Primary, &Rules);
    window.click(SlotRef::player(ARMOR.start), Click::Primary, &Rules);
    assert!(
        window.player.slots.get(ARMOR.start).is_none(),
        "dirt is no helmet"
    );
    window.click(SlotRef::player(0), Click::Primary, &Rules);
    window.click(SlotRef::player(1), Click::Transfer, &Rules);
    assert_eq!(window.player.slots.get(ARMOR.start).unwrap().item, HELMET);
    window.click(
        SlotRef::player(ARMOR.start + 1),
        Click::HotbarSwap(0),
        &Rules,
    );
    assert_eq!(
        window.player.slots.get(0).unwrap().item,
        DIRT,
        "dirt no chestplate"
    );
}

#[test]
fn shift_click_moves_between_rows_and_containers() {
    let mut player = with(&[(0, stack(DIRT, 10)), (12, stack(PEARL, 5))]);
    let mut window = Window {
        player: &mut player,
        open: None,
    };
    window.click(SlotRef::player(0), Click::Transfer, &Rules);
    assert_eq!(
        count(window.player.slots.get(MAIN.start)),
        10,
        "hotbar to main"
    );
    window.click(SlotRef::player(12), Click::Transfer, &Rules);
    assert_eq!(count(window.player.slots.get(0)), 5, "main to hotbar");

    let mut chest = Container::new(27);
    chest.slots[4] = Some(stack(DIRT, 60));
    let mut window = Window {
        player: &mut player,
        open: Some(&mut chest),
    };
    window.click(SlotRef::player(MAIN.start), Click::Transfer, &Rules);
    let chest_view = window.open.as_ref().unwrap();
    assert_eq!(
        (count(chest_view.get(4)), count(chest_view.get(0))),
        (64, 6)
    );
    window.click(SlotRef::open(4), Click::Transfer, &Rules);
    assert_eq!(window.player.slots.count(DIRT), 64);
}

#[test]
fn double_click_gathers_partial_stacks_first_and_skips_locked_ones() {
    let mut player = with(&[
        (0, stack(DIRT, 64)),
        (9, stack(DIRT, 5)),
        (10, stack(DIRT, 7)),
        (11, stack(DIRT, 9)),
    ]);
    player.toggle_lock(10);
    player.cursor = Some(stack(DIRT, 1));
    let mut window = Window {
        player: &mut player,
        open: None,
    };
    window.click(SlotRef::player(0), Click::Gather, &Rules);
    assert_eq!(count(window.player.cursor.as_ref()), 64);
    assert_eq!(
        count(window.player.slots.get(10)),
        7,
        "locked slot untouched"
    );
    assert!(window.player.slots.get(9).is_none() && window.player.slots.get(11).is_none());
    assert_eq!(
        count(window.player.slots.get(0)),
        15,
        "full stack used last"
    );
}

#[test]
fn drops_come_from_the_slot_or_the_cursor() {
    let mut player = with(&[(3, stack(DIRT, 8))]);
    let mut window = Window {
        player: &mut player,
        open: None,
    };
    let one = window.click(SlotRef::player(3), Click::Drop { whole: false }, &Rules);
    assert_eq!(one, vec![stack(DIRT, 1)]);
    window.click(SlotRef::player(3), Click::Primary, &Rules);
    let all = window.click(
        SlotRef::player(3),
        Click::DropCursor { whole: true },
        &Rules,
    );
    assert_eq!(all, vec![stack(DIRT, 7)]);
    assert!(window.player.cursor.is_none());
    assert!(
        window
            .click(SlotRef::player(99), Click::Primary, &Rules)
            .is_empty()
    );
}

#[test]
fn closing_puts_the_carried_stack_back() {
    let mut player = PlayerInventory {
        cursor: Some(stack(PEARL, 4)),
        ..PlayerInventory::default()
    };
    let mut window = Window {
        player: &mut player,
        open: None,
    };
    assert_eq!(window.close(&Rules), None);
    assert_eq!(player.slots.count(PEARL), 4);
}

#[test]
fn sorting_joins_orders_and_keeps_locked_slots() {
    let mut player = with(&[
        (9, stack(PEARL, 3)),
        (10, stack(DIRT, 20)),
        (11, stack(PEARL, 15)),
        (12, stack(DIRT, 50)),
    ]);
    player.toggle_lock(11);
    let locked = player.locked.clone();
    sort(&mut player.slots, MAIN, &locked, &Rules, |item| item.0);
    let slots = &player.slots;
    assert_eq!(slots.get(9), Some(&stack(DIRT, 64)));
    assert_eq!(slots.get(10), Some(&stack(DIRT, 6)));
    assert_eq!(slots.get(11), Some(&stack(PEARL, 15)), "locked, unmoved");
    assert_eq!(slots.get(12), Some(&stack(PEARL, 3)));
    assert!(slots.get(13).is_none());
}

#[test]
fn quick_deposit_moves_only_known_unlocked_items() {
    let mut player = with(&[
        (0, stack(DIRT, 10)),
        (9, stack(DIRT, 30)),
        (10, stack(PEARL, 5)),
        (11, stack(DIRT, 4)),
    ]);
    player.toggle_lock(11);
    let mut chest = Container::new(3);
    chest.slots[0] = Some(stack(DIRT, 1));
    assert_eq!(deposit_matching(&mut player, &mut chest, &Rules), 30);
    assert_eq!(chest.count(DIRT), 31);
    assert_eq!(player.slots.count(DIRT), 14, "hotbar and locked slots stay");
    assert_eq!(player.slots.count(PEARL), 5, "the chest holds no pearl");
}

#[test]
fn a_saved_inventory_reads_back() {
    let mut player = with(&[(0, stack(PICK, 1)), (40, stack(DIRT, 2))]);
    player.toggle_lock(0);
    let text = ron::to_string(&player).unwrap();
    let mut back: PlayerInventory = ron::from_str(&text).unwrap();
    back.normalize();
    assert_eq!(back, player);
    let bytes = postcard::to_stdvec(&player).unwrap();
    assert_eq!(
        postcard::from_bytes::<PlayerInventory>(&bytes).unwrap(),
        player
    );
}
