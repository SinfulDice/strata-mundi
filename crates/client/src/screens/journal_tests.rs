use super::*;
use std::path::Path;
use strata_core::inventory::{ItemStack, PlayerInventory};
use strata_core::pack::PackSet;
use strata_protocol::ServerMessage;

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    Content::load(&packs, &PackSet::default()).0
}

fn recipe(content: &Content, name: &str) -> RecipeId {
    content.recipes.key(&name.parse().unwrap()).unwrap()
}

#[test]
fn the_book_lists_learnt_recipes_filters_and_fills_the_grid() {
    let content = content();
    let mut crafting = ClientCrafting::default();
    let known: Vec<RecipeId> = content.recipes.starting().collect();
    crafting.receive(ServerMessage::Recipes { known, all: false });
    let planks = content
        .items
        .key(&"strata:oak_planks".parse().unwrap())
        .unwrap();
    let mut player = PlayerInventory::default();
    player.slots.slots[12] = Some(ItemStack::new(planks, 4, &content.items));
    let mut inventory = ClientInventory::default();
    inventory.restore(player);
    let mut state = JournalState::for_grid();
    let mut effects = Vec::new();
    let mut model = JournalModel::new(
        &mut state,
        Some(&mut inventory),
        Some(&crafting),
        &content,
        "en",
        &mut effects,
    );
    let all = listed(&model);
    let rows = model.rows("list");
    let chest = content.items.key(&"strata:chest".parse().unwrap()).unwrap();
    let table_item = content
        .items
        .key(&"strata:crafting_table".parse().unwrap())
        .unwrap();
    assert!(
        rows.iter().any(|row| row.item == Some(chest) && row.dim),
        "eight planks needed"
    );
    assert!(
        rows.iter()
            .any(|row| row.item == Some(table_item) && !row.dim)
    );
    assert!(all.contains(&recipe(&content, "strata:torch")));
    assert!(
        !all.contains(&recipe(&content, "strata:barrel")),
        "not learnt"
    );
    model.activate("craftable", 0);
    let craftable = listed(&model);
    let table = recipe(&content, "strata:crafting_table");
    assert!(craftable.contains(&table) && craftable.contains(&recipe(&content, "strata:stick")));
    assert!(
        !craftable.contains(&recipe(&content, "strata:chest")),
        "four planks only"
    );
    model.type_text("search", "crafting");
    assert_eq!(listed(&model), vec![table]);
    assert_eq!(
        model.rows("list")[0].item,
        Some(content.recipes.get(table).unwrap().result)
    );
    model.activate("list", 0);
    assert_eq!(model.items("pattern").iter().flatten().count(), 4);
    assert!(effects_empty(&model), "the first click only selects");
    model.activate("list", 0);
    assert!(popped(&model));
    assert_eq!(
        inventory.take_requests(),
        vec![InventoryRequest::FillRecipe {
            recipe: table,
            all: false
        }]
    );
}

#[test]
fn opened_alone_the_book_only_shows_recipes() {
    let content = content();
    let mut crafting = ClientCrafting::default();
    crafting.receive(ServerMessage::Recipes {
        known: Vec::new(),
        all: true,
    });
    let mut state = JournalState::default();
    let mut effects = Vec::new();
    let mut model = JournalModel::new(
        &mut state,
        None,
        Some(&crafting),
        &content,
        "fr",
        &mut effects,
    );
    assert_eq!(
        listed(&model).len(),
        content.recipes.len(),
        "creative knows all"
    );
    model.activate("list", 0);
    model.activate("list", 0);
    model.activate("fill_all", 0);
    assert!(effects_empty(&model));
    assert_eq!(model.tabs("tabs").len(), 1);
}

/// Recipes listed, by name.
fn listed(model: &JournalModel<'_>) -> Vec<RecipeId> {
    model.state.list.entries.iter().map(|(id, _)| *id).collect()
}

fn effects_empty(model: &JournalModel<'_>) -> bool {
    model.effects.is_empty()
}

fn popped(model: &JournalModel<'_>) -> bool {
    matches!(model.effects[..], [Effect::Pop])
}

#[test]
fn texts_are_composed_by_the_translations() {
    let content = content();
    let mut crafting = ClientCrafting::default();
    crafting.receive(ServerMessage::Recipes {
        known: Vec::new(),
        all: true,
    });
    let mut state = JournalState::default();
    let mut effects = Vec::new();
    let mut model = JournalModel::new(
        &mut state,
        None,
        Some(&crafting),
        &content,
        "en",
        &mut effects,
    );
    let chest = recipe(&content, "strata:chest");
    let index = listed(&model).iter().position(|&r| r == chest).unwrap();
    model.scroll("list", index as i32);
    let row = model.rows("list").into_iter().next().unwrap();
    assert_eq!(
        row.detail, "Needs a crafting table",
        "the inventory's grid is 2×2"
    );
    let first = model.state.scroll;
    model.activate("list", index - first);
    assert_eq!(model.text("detail").unwrap(), "8 × any planks");
    let torch = recipe(&content, "strata:torch");
    let index = listed(&model).iter().position(|&r| r == torch).unwrap();
    model.state.selected = Some(torch);
    assert_eq!(
        model.text("detail").unwrap(),
        "1 × coal or charcoal, 1 × Stick"
    );
    assert!(index < listed(&model).len());
}
