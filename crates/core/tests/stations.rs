//! The work stations of the base pack: each material at its station, the stations' own
//! recipes, their screens and help, and every bench recipe keeping the items it takes.
#![allow(clippy::unwrap_used)]

use std::path::Path;
use strata_core::content::{Content, ItemId, StationDef};
use strata_core::crafting::{BENCH_SIDE, BenchLayout, BenchRecipe, fill_bench, take_bench};
use strata_core::id::Identifier;
use strata_core::inventory::{Click, CraftGrid, ItemStack, PlayerInventory, Window};
use strata_core::lang::LANGUAGES;
use strata_core::pack::PackSet;

fn game() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let mut errors = Vec::new();
    let packs = PackSet::discover(&dir, &mut errors);
    let (content, more) = Content::load(&packs, &PackSet::default());
    errors.extend(more);
    assert!(errors.is_empty(), "{errors:#?}");
    content
}

fn id(text: &str) -> Identifier {
    text.parse().unwrap()
}

fn item(content: &Content, name: &str) -> ItemId {
    content.items.key(&id(name)).unwrap()
}

/// The bench that makes `name`, if one does.
fn made_at(content: &Content, name: &str) -> Option<String> {
    let key = item(content, name);
    let recipe = content.benches.list().iter().find(|r| r.result == key)?;
    Some(recipe.bench.to_string())
}

const STATIONS: [(&str, BenchLayout); 4] = [
    ("strata:forge", BenchLayout::Forge),
    ("strata:tannery", BenchLayout::Tannery),
    ("strata:loom", BenchLayout::Loom),
    ("strata:jeweller", BenchLayout::Jeweller),
];

#[test]
fn each_station_is_a_block_made_at_the_crafting_table_with_its_screen_and_help() {
    let content = game();
    for (name, layout) in STATIONS {
        let bench = id(name);
        let block = content.blocks.key(&bench).unwrap();
        let station = content.blocks.get(block).unwrap().station.clone();
        assert_eq!(station, Some(StationDef::Bench(bench.clone())), "{name}");
        let style = content.benches.style(&bench);
        assert_eq!(style.layout, layout, "{name}");
        assert!(style.accent.is_some(), "{name} has its colour");
        assert!(
            content.benches.of(&bench).count() >= 4,
            "{name} makes things"
        );
        let made = content.recipes.making(item(&content, name)).next();
        assert!(made.is_some(), "{name} is made at the crafting table");
        for language in LANGUAGES {
            let key = bench.translation_key("bench");
            assert!(
                content.translations.has(language, &key),
                "{language}: {key}"
            );
        }
    }
    let starting: Vec<String> = (content.recipes.starting())
        .filter_map(|r| content.recipes.identifier(r).map(ToString::to_string))
        .collect();
    assert!(starting.contains(&"strata:loom".to_owned()));
    assert!(starting.contains(&"strata:tannery".to_owned()));
    assert!(
        !starting.contains(&"strata:forge".to_owned()),
        "it needs iron"
    );
    let benches: Vec<String> = (content.benches.benches().into_iter())
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        benches,
        [
            "strata:forge",
            "strata:jeweller",
            "strata:loom",
            "strata:tannery"
        ]
    );
}

#[test]
fn each_material_has_its_station() {
    let content = game();
    let forge = |name: &str| (name.to_owned(), "strata:forge");
    let mut expected: Vec<(String, &str)> = Vec::new();
    for metal in ["iron_mail", "gold_mail"] {
        for piece in ["coif", "shirt", "leggings", "boots"] {
            expected.push(forge(&format!("strata:{metal}_{piece}")));
        }
    }
    for metal in ["iron_plate", "gold_plate", "diamond_plate", "diamond_scale"] {
        for piece in ["helmet", "chestplate", "leggings", "boots"] {
            expected.push(forge(&format!("strata:{metal}_{piece}")));
        }
    }
    for made in ["iron_rings", "gold_rings", "diamond_scales"] {
        expected.push(forge(&format!("strata:{made}")));
    }
    for boots in ["leaping_boots", "crampon_boots"] {
        expected.push(forge(&format!("strata:{boots}")));
    }
    for piece in ["leather", "leather_cap", "leather_tunic", "leather_pants"] {
        expected.push((format!("strata:{piece}"), "strata:tannery"));
    }
    expected.push(("strata:leather_boots".into(), "strata:tannery"));
    for piece in [
        "cloth",
        "wool",
        "cloth_hood",
        "cloth_tunic",
        "cloth_trousers",
    ] {
        expected.push((format!("strata:{piece}"), "strata:loom"));
    }
    expected.push(("strata:cloth_shoes".into(), "strata:loom"));
    expected.push(("strata:wool_cloak".into(), "strata:loom"));
    for accessory in ["ring_of_vitality", "amulet_of_short_breath", "vial_belt"] {
        expected.push((format!("strata:{accessory}"), "strata:jeweller"));
    }
    for (name, bench) in expected {
        assert_eq!(made_at(&content, &name).as_deref(), Some(bench), "{name}");
        let key = item(&content, &name);
        let in_grid = content.recipes.making(key).next();
        assert!(in_grid.is_none(), "{name} is no longer made in a grid");
    }
    let coals = id("strata:coals");
    let scales = item(&content, "strata:diamond_scales");
    let forge = id("strata:forge");
    let metal = (content.benches.of(&forge)).filter(|(_, r)| r.result != scales);
    for (_, recipe) in metal {
        let burns =
            (recipe.ingredients.iter()).any(|(i, n)| i.tag.as_ref() == Some(&coals) && *n == 1);
        assert!(burns, "{:?} burns one coal", recipe.id);
    }
}

/// Lays one craft's worth of each ingredient (twice over) in an inventory, chooses the
/// recipe, takes the result: exactly what it takes is gone, the result is there.
fn craft_once(content: &Content, recipe: &BenchRecipe) {
    let rules = &content.items;
    let mut player = PlayerInventory::default();
    let mut wanted: Vec<(ItemId, u32)> = Vec::new();
    for (slot, (ingredient, count)) in recipe.ingredients.iter().enumerate() {
        let item = *ingredient.items().first().unwrap();
        let stack = ItemStack::new(item, *count, rules);
        player.slots.slots[9 + 2 * slot] = Some(stack.clone());
        player.slots.slots[10 + 2 * slot] = Some(stack);
        wanted.push((item, u32::from(*count)));
    }
    let before: Vec<u32> = wanted.iter().map(|(i, _)| player.slots.count(*i)).collect();
    let mut cells = CraftGrid::new(BENCH_SIDE);
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut cells),
        policy: None,
    };
    assert!(fill_bench(&mut window, recipe, rules), "{}", recipe.id);
    let (times, thrown) = take_bench(&mut window, recipe, Click::Primary, rules);
    assert_eq!((times, thrown.len()), (1, 0), "{}", recipe.id);
    let made = window.player.cursor.take().unwrap();
    assert_eq!((made.item, made.count), (recipe.result, recipe.count));
    for ((item, count), had) in wanted.iter().zip(before) {
        let left = player.slots.count(*item) + cells.slots.count(*item);
        assert_eq!(left, had - count, "{}: {item:?}", recipe.id);
    }
}

#[test]
fn every_bench_recipe_takes_exactly_its_ingredients_and_gives_its_result() {
    let content = game();
    for recipe in content.benches.list() {
        craft_once(&content, recipe);
    }
}
