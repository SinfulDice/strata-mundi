use super::tests::{BIRCH, BUCKET, DIRT, OAK, TORCH, WATER, id, items};
use super::*;
use crate::content::ItemId;
use crate::inventory::{Click, CraftGrid, ItemStack, PlayerInventory, Window};

fn recipe(ingredients: Vec<(Ingredient, u16)>) -> BenchRecipe {
    BenchRecipe {
        id: id("r"),
        bench: id("jeweller"),
        ingredients,
        result: TORCH,
        count: 1,
    }
}

fn grid(stacks: &[(ItemId, u16)]) -> CraftGrid {
    let items = items();
    let mut grid = CraftGrid::new(BENCH_SIDE);
    for (cell, &(item, count)) in stacks.iter().enumerate() {
        grid.slots.slots[cell] = Some(ItemStack::new(item, count, &items));
    }
    grid
}

#[test]
fn a_named_item_picks_its_cell_before_a_tag() {
    let planks = Ingredient::new(vec![BIRCH, OAK], Some(id("planks")));
    let wanted = recipe(vec![(planks, 1), (Ingredient::item(OAK), 1)]);
    let laid = grid(&[(OAK, 1), (BIRCH, 1)]);
    assert_eq!(wanted.cells(&laid), Some(vec![1, 0]));
}

#[test]
fn remainders_stay_in_the_bench_like_in_a_grid() {
    let items = items();
    let wanted = recipe(vec![
        (Ingredient::item(WATER), 1),
        (Ingredient::item(DIRT), 2),
    ]);
    let mut player = PlayerInventory::default();
    let mut cells = grid(&[(WATER, 1), (DIRT, 4)]);
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut cells),
        policy: None,
    };
    let (times, thrown) = take_bench(&mut window, &wanted, Click::Transfer, &items);
    assert_eq!(times, 1, "one water only");
    assert!(thrown.is_empty());
    assert_eq!(cells.slots.get(0).map(|s| s.item), Some(BUCKET));
    assert_eq!(cells.slots.get(1).map(|s| s.count), Some(2));
    assert_eq!(player.slots.count(TORCH), 1);
}

#[test]
fn can_lay_answers_like_fill_bench() {
    let items = items();
    let wanted = recipe(vec![(Ingredient::item(DIRT), 3)]);
    let mut player = PlayerInventory::default();
    player.slots.slots[0] = Some(ItemStack::new(DIRT, 2, &items));
    let mut cells = CraftGrid::new(BENCH_SIDE);
    assert!(!can_lay(&player, &cells, &wanted, &items));
    player.slots.slots[5] = Some(ItemStack::new(DIRT, 1, &items));
    assert!(can_lay(&player, &cells, &wanted, &items));
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut cells),
        policy: None,
    };
    assert!(fill_bench(&mut window, &wanted, &items));
    assert_eq!(cells.slots.get(0).map(|s| s.count), Some(3));
    assert_eq!(player.slots.count(DIRT), 0);
}
