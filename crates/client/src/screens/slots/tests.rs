#![allow(clippy::unwrap_used)]

use super::*;
use std::path::Path;
use strata_core::coords::BlockPos;
use strata_core::inventory::{Container, PlayerInventory};
use strata_core::pack::PackSet;

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    Content::load(&packs, &PackSet::default()).0
}

fn item(content: &Content, name: &str) -> ItemId {
    content.items.key(&name.parse().unwrap()).unwrap()
}

#[test]
fn grids_name_the_right_slots_and_clicks_become_requests() {
    let content = content();
    let stone = item(&content, "strata:stone");
    let mut inventory = PlayerInventory::default();
    inventory.slots.slots[MAIN.start + 2] = Some(ItemStack::new(stone, 7, &content.items));
    inventory.toggle_lock(MAIN.start + 2);
    let mut hotbar = Hotbar::default();
    hotbar.restore(inventory);
    let mut chest = Container::new(27);
    chest.slots[3] = Some(ItemStack::new(stone, 2, &content.items));
    let open = OpenedContainer {
        block: strata_core::content::BlockId(1),
        pos: BlockPos::new(0, 0, 0),
        contents: chest,
    };
    let mut state = SlotsState {
        search: "zzz".into(),
        for_container: true,
    };
    let mut model = SlotsModel {
        state: &mut state,
        hotbar: &mut hotbar,
        container: Some(&open),
        content: &content,
        language: "en",
    };
    assert_eq!(model.items("main")[2], Some(stone));
    let mark = model.marks("main")[2];
    assert_eq!((mark.count, mark.locked, mark.dim), (7, true, false));
    assert!(model.marks("chest")[3].dim, "the search leaves it out");
    assert_eq!(model.items("chest").len(), 27);
    assert_eq!(model.items("armor").len(), 4);
    assert_eq!(model.slot("chest", 30), None, "past the chest's slots");
    assert!(model.press("main", 2, SlotPress::Click(Click::Transfer)));
    model.activate("deposit", 0);
    model.quick_slot("chest", 3, 4);
    let requests = hotbar.take_requests();
    assert_eq!(
        requests,
        vec![
            InventoryRequest::Click {
                slot: SlotRef::player(MAIN.start + 2),
                click: Click::Transfer,
            },
            InventoryRequest::QuickDeposit,
            InventoryRequest::Click {
                slot: SlotRef::open(3),
                click: Click::HotbarSwap(4),
            },
        ]
    );
}
