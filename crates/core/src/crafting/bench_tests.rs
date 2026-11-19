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

fn write_pack(dir: &std::path::Path, name: &str, priority: i32, files: &[(&str, &str)]) {
    let root = dir.join(name);
    std::fs::create_dir_all(root.join("benches/base")).unwrap();
    let manifest = format!("(namespace: \"{name}\", priority: {priority})");
    std::fs::write(root.join("pack.ron"), manifest).unwrap();
    for (path, text) in files {
        std::fs::write(root.join(path), text).unwrap();
    }
}

#[test]
fn bench_styles_load_layer_and_name_a_bench_of_a_block() {
    use crate::crafting::bench_style::load_styles;
    let dir = std::env::temp_dir().join(format!("strata-benches-{}", std::process::id()));
    let base = [
        (
            "benches/forge.ron",
            "Bench(layout: \"forge\", accent: \"#7a3b1e\")",
        ),
        ("benches/loom.ron", "Bench(layout: \"loom\")"),
        ("benches/odd.ron", "Bench()"),
    ];
    write_pack(&dir, "base", 0, &base);
    let over = [
        ("benches/base/forge.ron", "Bench(accent: \"#102030\")"),
        ("remove.ron", "{ \"benches\": [\"base:loom\"] }"),
    ];
    write_pack(&dir, "over", 10, &over);
    let mut errors = Vec::new();
    let packs = crate::pack::PackSet::discover(&dir, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    let stations: std::collections::BTreeSet<crate::id::Identifier> = ["base:forge", "base:loom"]
        .map(|s| s.parse().unwrap())
        .into();
    let styles = load_styles(&packs, &stations, &mut errors);
    assert_eq!(errors.len(), 1, "no block opens base:odd: {errors:?}");
    let mut benches = Benches::new(Vec::new()).with_styles(styles);
    let forge = benches.style(&"base:forge".parse().unwrap()).clone();
    assert_eq!(
        forge.layout_name(),
        "plain",
        "the later pack replaces the file"
    );
    assert_eq!(forge.accent.map(|c| c.0.0), Some([0x10, 0x20, 0x30, 0xff]));
    let loom = benches.style(&"base:loom".parse().unwrap());
    assert_eq!(*loom, BenchStyle::PLAIN, "removed by remove.ron");
    std::fs::remove_dir_all(&dir).unwrap();
    write_pack(&dir, "base", 0, &base[..1]);
    let packs = crate::pack::PackSet::discover(&dir, &mut errors);
    benches = Benches::new(Vec::new()).with_styles(load_styles(&packs, &stations, &mut errors));
    let dropped = benches.drop_layouts(|layout| layout == "forge");
    assert!(dropped.is_empty());
    let dropped = benches.drop_layouts(|layout| layout == "plain");
    assert_eq!(
        dropped,
        [("base:forge".parse().unwrap(), "forge".to_owned())]
    );
    assert_eq!(
        benches.style(&"base:forge".parse().unwrap()).layout_name(),
        "plain"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
