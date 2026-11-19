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
    let mut hotbar = ClientInventory::default();
    hotbar.restore(inventory);
    let mut chest = Container::new(27);
    chest.slots[3] = Some(ItemStack::new(stone, 2, &content.items));
    let open = OpenedContainer {
        block: strata_core::content::BlockId(1),
        pos: BlockPos::new(0, 0, 0),
        contents: chest,
        upgrades_at: 27,
        furnace: None,
    };
    let mut state = SlotsState {
        search: "zzz".into(),
        for_container: true,
        ..Default::default()
    };
    let mut effects = Vec::new();
    let mut model = SlotsModel {
        state: &mut state,
        inventory: &mut hotbar,
        container: Some(&open),
        crafting: None,
        effects: &mut effects,
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

#[test]
fn the_anvil_lists_what_the_item_laid_can_take() {
    let content = content();
    let anvil = content
        .blocks
        .key(&"strata:anvil".parse().unwrap())
        .unwrap();
    let sword = item(&content, "strata:iron_sword");
    let mut crafting = ClientCrafting::default();
    crafting.view.station = Some((anvil, BlockPos::new(0, 0, 0)));
    crafting.view.grid = Container::new(2);
    crafting.view.grid.slots[0] = Some(ItemStack::new(sword, 1, &content.items));
    let mut hotbar = ClientInventory::default();
    let mut state = SlotsState::default();
    let mut effects = Vec::new();
    let mut model = SlotsModel {
        state: &mut state,
        inventory: &mut hotbar,
        container: None,
        crafting: Some(&crafting),
        effects: &mut effects,
        content: &content,
        language: "fr",
    };
    assert!(model.shown("anvil_side") && !model.shown("table_side"));
    let copper = item(&content, "strata:copper_ingot");
    let iron = item(&content, "strata:iron_ingot");
    assert_eq!(model.items("anvil_choices"), [Some(iron), Some(copper)]);
    let tooltip = model.tooltip("anvil_choices", 1).unwrap();
    assert!(
        tooltip.contains("Aiguisage 1") && tooltip.contains("2 XP"),
        "{tooltip}"
    );
    assert_eq!(model.slot("anvil", 1).map(|s| s.index), Some(1));
    assert!(!model.shown("anvil_scroll"), "two upgrades fit");
    model.scroll("anvil_choices", 3);
    assert_eq!(model.items("anvil_choices").len(), 2, "nothing to scroll");
    let tip = model.tooltip("anvil", 1).unwrap();
    assert!(
        tip.contains("matériau"),
        "an empty cell says what goes there: {tip}"
    );
    assert!(model.press("anvil_choices", 1, SlotPress::Click(Click::Primary)));
    model.activate("anvil_repair", 0);
    let sharpening = content
        .upgrades
        .index_of(&"strata:sharpening".parse().unwrap());
    assert_eq!(
        hotbar.take_requests(),
        [
            InventoryRequest::ChooseUpgrade(sharpening),
            InventoryRequest::ChooseUpgrade(None)
        ]
    );
}
