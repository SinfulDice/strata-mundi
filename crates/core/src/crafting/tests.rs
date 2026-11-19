use super::*;
use crate::content::{Item, ItemId, ItemKind, Registry, TextureId};
use crate::id::Identifier;
use crate::inventory::{Click, CraftGrid, ItemStack, PlayerInventory, Window};

pub(super) const LOG: ItemId = ItemId(0);
pub(super) const OAK: ItemId = ItemId(1);
pub(super) const BIRCH: ItemId = ItemId(2);
pub(super) const STICK: ItemId = ItemId(3);
pub(super) const TABLE: ItemId = ItemId(4);
pub(super) const WATER: ItemId = ItemId(5);
pub(super) const BUCKET: ItemId = ItemId(6);
pub(super) const DIRT: ItemId = ItemId(7);
pub(super) const MUD: ItemId = ItemId(8);
pub(super) const COAL: ItemId = ItemId(9);
pub(super) const TORCH: ItemId = ItemId(10);

pub(super) fn id(name: &str) -> Identifier {
    Identifier::new("test", name).unwrap()
}

pub(super) fn items() -> Registry<ItemId, Item> {
    let mut items = Registry::default();
    let names = [
        "log", "oak", "birch", "stick", "table", "water", "bucket", "dirt", "mud", "coal", "torch",
    ];
    for name in names {
        let item = Item {
            max_stack: if name == "water" { 1 } else { 64 },
            category: None,
            kind: ItemKind::Flat(TextureId::MISSING),
            action: None,
            durability: None,
            equip: None,
            tool: None,
            remainder: (name == "water").then_some(BUCKET),
            kit: None,
            upgrade: None,
            gear: Default::default(),
        };
        items.insert(id(name), item);
    }
    items
}

fn planks() -> Ingredient {
    Ingredient::new(vec![BIRCH, OAK], Some(id("planks")))
}

fn shaped(width: usize, cells: Vec<Option<Ingredient>>, result: ItemId, count: u16) -> Recipe {
    Recipe {
        shape: RecipeShape::Shaped {
            width,
            height: cells.len() / width,
            cells,
        },
        result,
        count,
        starting: true,
    }
}

fn recipes() -> Recipes {
    let item = |i| Some(Ingredient::item(i));
    let list = vec![
        (
            id("oak"),
            Recipe {
                shape: RecipeShape::Shapeless(vec![Ingredient::item(LOG)]),
                result: OAK,
                count: 4,
                starting: true,
            },
        ),
        (
            id("stick"),
            shaped(1, vec![Some(planks()), Some(planks())], STICK, 4),
        ),
        (id("table"), shaped(2, vec![Some(planks()); 4], TABLE, 1)),
        (
            id("torch"),
            shaped(2, vec![item(COAL), None, None, item(STICK)], TORCH, 4),
        ),
        (
            id("mud"),
            Recipe {
                shape: RecipeShape::Shapeless(vec![
                    Ingredient::item(DIRT),
                    Ingredient::item(DIRT),
                    Ingredient::item(WATER),
                ]),
                result: MUD,
                count: 2,
                starting: false,
            },
        ),
    ];
    Recipes::new(list)
}

fn find(recipes: &Recipes, side: usize, cells: &[Option<ItemId>]) -> Option<String> {
    let found = recipes.find(GridItems { side, cells })?;
    Some(recipes.identifier(found)?.name().to_owned())
}

#[test]
fn shaped_recipes_are_found_anywhere_with_tags() {
    let recipes = recipes();
    let n = None;
    assert_eq!(
        find(&recipes, 2, &[Some(OAK), n, Some(BIRCH), n]).as_deref(),
        Some("stick")
    );
    let low_right = [n, n, n, n, Some(OAK), n, n, Some(OAK), n];
    assert_eq!(find(&recipes, 3, &low_right).as_deref(), Some("stick"));
    let mixed = [Some(OAK), Some(BIRCH), Some(BIRCH), Some(OAK)];
    assert_eq!(find(&recipes, 2, &mixed).as_deref(), Some("table"));
    assert_eq!(find(&recipes, 2, &[Some(OAK), Some(OAK), n, n]), None);
    assert_eq!(find(&recipes, 2, &[Some(LOG), n, n, Some(STICK)]), None);
}

#[test]
fn shaped_recipes_also_match_mirrored() {
    let recipes = recipes();
    let n = None;
    assert_eq!(
        find(&recipes, 2, &[Some(COAL), n, n, Some(STICK)]).as_deref(),
        Some("torch")
    );
    assert_eq!(
        find(&recipes, 2, &[n, Some(COAL), Some(STICK), n]).as_deref(),
        Some("torch")
    );
    assert_eq!(
        find(&recipes, 2, &[Some(STICK), n, n, Some(COAL)]),
        None,
        "upside down"
    );
}

#[test]
fn shapeless_recipes_take_any_cells_in_any_order() {
    let recipes = recipes();
    let n = None;
    let cells = [n, Some(WATER), n, Some(DIRT), n, n, n, n, Some(DIRT)];
    assert_eq!(find(&recipes, 3, &cells).as_deref(), Some("mud"));
    assert_eq!(
        find(&recipes, 2, &[n, n, Some(LOG), n]).as_deref(),
        Some("oak")
    );
    assert_eq!(
        find(&recipes, 2, &[Some(LOG), n, Some(LOG), n]),
        None,
        "one log too many"
    );
    assert_eq!(find(&recipes, 2, &[n; 4]), None);
}

#[test]
fn a_hint_shows_for_part_of_an_unknown_recipe() {
    let recipes = recipes();
    let n = None;
    let mud = recipes.key(&id("mud")).unwrap();
    let unknown = |r| r != mud;
    let grid = |cells: &[Option<ItemId>]| GridItems { side: 3, cells }.cells.to_vec();
    let partial = grid(&[Some(DIRT), n, n, n, n, n, n, n, Some(WATER)]);
    assert!(recipes.hint(
        GridItems {
            side: 3,
            cells: &partial
        },
        unknown
    ));
    assert!(
        !recipes.hint(
            GridItems {
                side: 3,
                cells: &partial
            },
            |_| true
        ),
        "known"
    );
    let small = [Some(DIRT), n, n, Some(WATER)];
    assert!(
        recipes.hint(
            GridItems {
                side: 2,
                cells: &small
            },
            unknown
        ),
        "three fit in 2×2"
    );
    let stray = grid(&[Some(DIRT), Some(TORCH), n, n, n, n, n, n, n]);
    assert!(!recipes.hint(
        GridItems {
            side: 3,
            cells: &stray
        },
        unknown
    ));
}

fn stack(item: ItemId, count: u16) -> ItemStack {
    ItemStack::new(item, count, &items())
}

fn grid(side: u8, cells: &[(usize, ItemStack)]) -> CraftGrid {
    let mut grid = CraftGrid::new(side);
    for (index, stack) in cells {
        grid.slots.slots[*index] = Some(stack.clone());
    }
    grid
}

fn total(player: &PlayerInventory, grid: &CraftGrid, item: ItemId) -> u32 {
    let carried = player.cursor.iter().filter(|s| s.item == item);
    player.slots.count(item)
        + grid.slots.count(item)
        + carried.map(|s| u32::from(s.count)).sum::<u32>()
}

#[test]
fn a_click_crafts_once_into_the_carried_stack() {
    let (recipes, items) = (recipes(), items());
    let mut player = PlayerInventory::default();
    let mut grid = grid(2, &[(0, stack(OAK, 3)), (2, stack(BIRCH, 1))]);
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    let crafted = take_result(&mut window, &recipes, Click::Primary, &items).unwrap();
    assert_eq!(crafted.times, 1);
    assert_eq!(window.player.cursor, Some(stack(STICK, 4)));
    assert!(
        take_result(&mut window, &recipes, Click::Primary, &items).is_none(),
        "birch used"
    );
    assert_eq!(grid.slots.get(0), Some(&stack(OAK, 2)));
    assert_eq!(grid.slots.get(2), None);
}

#[test]
fn shift_click_crafts_as_many_as_possible_and_keeps_every_item() {
    let (recipes, items) = (recipes(), items());
    let mut player = PlayerInventory::default();
    let mut grid = grid(2, &[(1, stack(LOG, 20))]);
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    let crafted = take_result(&mut window, &recipes, Click::Transfer, &items).unwrap();
    assert_eq!(crafted.times, 20);
    assert_eq!(player.slots.count(OAK), 80);
    assert!(grid.slots.is_empty());
}

#[test]
fn shift_click_stops_when_the_inventory_is_full() {
    let (recipes, items) = (recipes(), items());
    let mut player = PlayerInventory::default();
    for slot in player.slots.slots.iter_mut().take(36) {
        *slot = Some(stack(DIRT, 64));
    }
    player.slots.slots[3] = Some(stack(OAK, 56));
    let mut grid = grid(2, &[(0, stack(LOG, 5))]);
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    let crafted = take_result(&mut window, &recipes, Click::Transfer, &items).unwrap();
    assert_eq!(crafted.times, 2);
    assert_eq!(
        total(&player, &grid, LOG) * 4 + total(&player, &grid, OAK),
        5 * 4 + 56
    );
}

#[test]
fn remainders_stay_in_the_grid() {
    let (recipes, items) = (recipes(), items());
    let mut player = PlayerInventory::default();
    let mut grid = grid(
        3,
        &[
            (0, stack(DIRT, 4)),
            (4, stack(WATER, 1)),
            (8, stack(DIRT, 2)),
        ],
    );
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    let crafted = take_result(&mut window, &recipes, Click::Transfer, &items).unwrap();
    assert_eq!(crafted.times, 1, "the water is gone after one");
    assert!(crafted.thrown.is_empty());
    assert_eq!(grid.slots.get(4), Some(&stack(BUCKET, 1)));
    assert_eq!(player.slots.count(MUD), 2);
    assert_eq!(grid.slots.count(DIRT), 4);
}

#[test]
fn a_full_cursor_of_something_else_crafts_nothing() {
    let (recipes, items) = (recipes(), items());
    let mut player = PlayerInventory {
        cursor: Some(stack(DIRT, 1)),
        ..PlayerInventory::default()
    };
    let mut grid = grid(2, &[(0, stack(LOG, 1))]);
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    assert!(take_result(&mut window, &recipes, Click::Primary, &items).is_none());
    assert_eq!(grid.slots.count(LOG), 1);
}

#[test]
fn plans_count_what_the_inventory_holds() {
    let recipes = recipes();
    let table = recipes.get(recipes.key(&id("table")).unwrap()).unwrap();
    let held = [stack(OAK, 3), stack(BIRCH, 1)];
    let mut available = counts(held.iter());
    assert!(can_make(table, &mut available));
    assert_eq!(
        available,
        counts(held.iter()),
        "the counts are left as they were"
    );
    assert!(!can_make(table, &mut counts(held[..1].iter())));
    assert_eq!(plan(table, &mut available).unwrap().len(), 4);
}

#[test]
fn the_recipe_book_fills_the_grid_from_the_inventory() {
    let (recipes, items) = (recipes(), items());
    let torch = recipes.get(recipes.key(&id("torch")).unwrap()).unwrap();
    let mut player = PlayerInventory::default();
    player.slots.slots[0] = Some(stack(COAL, 3));
    player.slots.slots[10] = Some(stack(STICK, 5));
    let mut grid = grid(3, &[(4, stack(DIRT, 2))]);
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    assert!(fill(&mut window, torch, false, &items));
    assert_eq!(grid.slots.get(0), Some(&stack(COAL, 1)));
    assert_eq!(grid.slots.get(4), Some(&stack(STICK, 1)));
    assert_eq!(player.slots.count(DIRT), 2, "the grid's stacks went back");
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    assert!(fill(&mut window, torch, true, &items));
    assert_eq!(grid.slots.get(0), Some(&stack(COAL, 3)));
    assert_eq!(grid.slots.get(4), Some(&stack(STICK, 3)));
    assert_eq!(player.slots.count(STICK), 2);
    assert_eq!(player.slots.count(COAL), 0);
}

#[test]
fn the_recipe_book_changes_nothing_without_the_items_or_the_room() {
    let (recipes, items) = (recipes(), items());
    let mud = recipes.get(recipes.key(&id("mud")).unwrap()).unwrap();
    let mut player = PlayerInventory::default();
    player.slots.slots[0] = Some(stack(DIRT, 2));
    let mut grid = grid(3, &[(4, stack(WATER, 1))]);
    let before = (player.clone(), grid.clone());
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    assert!(
        fill(&mut window, mud, false, &items),
        "the water came back from the grid"
    );
    let mut small = CraftGrid::new(2);
    let mut window = Window {
        player: &mut before.0.clone(),
        open: None,
        craft: Some(&mut small),
        policy: None,
    };
    let three_by_three = shaped(3, vec![Some(Ingredient::item(DIRT)); 3], MUD, 1);
    assert!(!fill(&mut window, &three_by_three, false, &items));
    let mut window = Window {
        player: &mut PlayerInventory::default(),
        open: None,
        craft: Some(&mut before.1.clone()),
        policy: None,
    };
    assert!(
        !fill(&mut window, &three_by_three, false, &items),
        "nothing to lay"
    );
}
