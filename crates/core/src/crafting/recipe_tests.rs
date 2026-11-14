use super::*;
use crate::content::{ItemKind, TextureId};

fn items() -> Registry<ItemId, Item> {
    let mut items = Registry::default();
    for (name, max_stack) in [("a", 64), ("b", 64), ("tool", 1)] {
        let item = Item {
            max_stack,
            category: None,
            kind: ItemKind::Flat(TextureId::MISSING),
            action: None,
            durability: None,
            equip: None,
            tool: None,
            remainder: None,
        };
        items.insert(format!("test:{name}").parse().unwrap(), item);
    }
    items
}

fn shaped(pattern: &[&str], result: &str, count: u16) -> RecipeDef {
    RecipeDef::Shaped {
        pattern: pattern.iter().map(|row| (*row).to_owned()).collect(),
        key: BTreeMap::from([
            ('a', IngredientDef::Item("test:a".parse().unwrap())),
            ('b', IngredientDef::Item("test:b".parse().unwrap())),
        ]),
        result: ResultDef {
            item: result.parse().unwrap(),
            count,
        },
        starting: false,
    }
}

fn load(def: RecipeDef) -> (Option<Recipe>, Vec<PackError>) {
    let items = items();
    let tags = ItemTags::default();
    let names = Names {
        items: &items,
        tags: &tags,
    };
    let mut errors = Vec::new();
    let recipe = resolve(def, &names, Path::new("recipes/x.ron"), &mut errors);
    (recipe, errors)
}

#[test]
fn empty_outer_rows_and_columns_are_trimmed() {
    let (recipe, errors) = load(shaped(&["   ", " a ", " b "], "test:a", 1));
    assert!(errors.is_empty(), "{errors:?}");
    let recipe = recipe.unwrap();
    let RecipeShape::Shaped { width, height, .. } = &recipe.shape else {
        panic!("shaped");
    };
    assert_eq!((*width, *height), (1, 2));
    assert_eq!(recipe.grid_size(), 2);
    let cells: Vec<usize> = recipe.cells_in(3).iter().map(|(cell, _)| *cell).collect();
    assert_eq!(cells, vec![0, 3]);
    let (inner_gap, _) = load(shaped(&["a b"], "test:a", 1));
    assert_eq!(inner_gap.unwrap().grid_size(), 3, "the gap between stays");
}

#[test]
fn results_are_kept_within_a_stack_and_unknown_names_reported() {
    let (recipe, errors) = load(shaped(&["a"], "test:tool", 4));
    assert_eq!(recipe.unwrap().count, 1, "a tool does not stack");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].to_string().contains("recipes/x.ron"));
    let (recipe, errors) = load(shaped(&["c"], "test:a", 1));
    assert!(recipe.is_none());
    assert!(errors[0].to_string().contains("`c` is not in the key"));
    let (recipe, errors) = load(shaped(&["a"], "test:nope", 1));
    assert!(recipe.is_none() && errors[0].to_string().contains("test:nope"));
}
