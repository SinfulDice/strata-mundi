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
    let mut state = JournalState {
        for_grid: true,
        ..JournalState::default()
    };
    let mut effects = Vec::new();
    let mut model = JournalModel {
        state: &mut state,
        inventory: Some(&mut inventory),
        crafting: Some(&crafting),
        content: &content,
        language: "en",
        effects: &mut effects,
    };
    let listed = model.recipes();
    assert!(listed.contains(&recipe(&content, "strata:torch")));
    assert!(
        !listed.contains(&recipe(&content, "strata:barrel")),
        "not learnt"
    );
    model.activate("craftable", 0);
    let craftable = model.recipes();
    let table = recipe(&content, "strata:crafting_table");
    assert!(craftable.contains(&table) && craftable.contains(&recipe(&content, "strata:stick")));
    assert!(
        !craftable.contains(&recipe(&content, "strata:chest")),
        "four planks only"
    );
    model.type_text("search", "crafting");
    assert_eq!(model.recipes(), vec![table]);
    assert_eq!(
        model.rows("list")[0].item,
        Some(content.recipes.get(table).unwrap().result)
    );
    model.activate("list", 0);
    assert_eq!(model.items("pattern").iter().flatten().count(), 4);
    assert!(model.effects.is_empty(), "the first click only selects");
    model.activate("list", 0);
    assert!(matches!(model.effects[..], [Effect::Pop]));
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
    let mut model = JournalModel {
        state: &mut state,
        inventory: None,
        crafting: Some(&crafting),
        content: &content,
        language: "fr",
        effects: &mut effects,
    };
    assert_eq!(
        model.recipes().len(),
        content.recipes.len(),
        "creative knows all"
    );
    model.activate("list", 0);
    model.activate("list", 0);
    model.activate("fill_all", 0);
    assert!(model.effects.is_empty());
    assert_eq!(model.tabs("tabs").len(), 1);
}
