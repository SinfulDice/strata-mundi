//! The tools, weapons and armour of the base pack: tiers, recipes, stats and tooltips.
#![allow(clippy::unwrap_used)]

use std::path::Path;
use strata_core::content::{Content, ItemId};
use strata_core::crafting::grid_result;
use strata_core::id::Identifier;
use strata_core::inventory::{CraftGrid, ItemStack};
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

/// What a 3×3 grid laid out as `pattern` (`.` empty) makes.
fn craft(content: &Content, pattern: [&str; 3], key: &[(char, &str)]) -> Option<String> {
    let mut grid = CraftGrid::new(3);
    for (row, line) in pattern.iter().enumerate() {
        for (column, symbol) in line.chars().enumerate() {
            let Some(&(_, name)) = key.iter().find(|(k, _)| *k == symbol) else {
                continue;
            };
            let stack = ItemStack::new(item(content, name), 1, &content.items);
            grid.slots.slots[row * 3 + column] = Some(stack);
        }
    }
    let result = grid_result(&content.recipes, &grid, &content.items)?;
    Some(content.items.identifier(result.item)?.to_string())
}

#[test]
fn tiers_go_up_in_level_and_leave_room_for_more() {
    let content = game();
    let level = |name: &str| content.gear.tiers.get(&id(name)).unwrap().level;
    assert_eq!(
        [
            "strata:wood",
            "strata:stone",
            "strata:iron",
            "strata:diamond"
        ]
        .map(level),
        [0, 1, 2, 3]
    );
    let slots = |name: &str| {
        content
            .gear
            .tiers
            .get(&id(name))
            .unwrap()
            .upgrade_slots
            .len()
    };
    assert!(slots("strata:wood") <= slots("strata:iron"));
    assert!(slots("strata:iron") < slots("strata:diamond"));
    assert!(content.gear.tiers.values().all(|tier| tier.level < u8::MAX));
}

#[test]
fn tool_recipes_follow_the_tiers_and_wood_and_stone_ones_are_known_at_start() {
    let content = game();
    let pickaxe = |head| {
        craft(
            &content,
            ["MMM", ".S.", ".S."],
            &[('M', head), ('S', "strata:stick")],
        )
    };
    assert_eq!(
        pickaxe("strata:birch_planks").as_deref(),
        Some("strata:wooden_pickaxe")
    );
    assert_eq!(
        pickaxe("strata:cobblestone").as_deref(),
        Some("strata:stone_pickaxe")
    );
    assert_eq!(
        pickaxe("strata:iron_ingot").as_deref(),
        Some("strata:iron_pickaxe")
    );
    assert_eq!(
        pickaxe("strata:diamond").as_deref(),
        Some("strata:diamond_pickaxe")
    );
    let starting: Vec<String> = content
        .recipes
        .starting()
        .filter_map(|r| content.recipes.identifier(r).map(ToString::to_string))
        .collect();
    for tool in ["pickaxe", "axe", "shovel", "hoe", "sword"] {
        assert!(
            starting.contains(&format!("strata:wooden_{tool}")),
            "{tool}"
        );
        assert!(starting.contains(&format!("strata:stone_{tool}")), "{tool}");
        assert!(!starting.contains(&format!("strata:iron_{tool}")), "{tool}");
    }
}

#[test]
fn mail_and_plate_come_from_rings_and_ingots() {
    let content = game();
    let chest = |metal| craft(&content, ["M.M", "MMM", "MMM"], &[('M', metal)]);
    assert_eq!(
        chest("strata:iron_rings").as_deref(),
        Some("strata:iron_mail_shirt")
    );
    assert_eq!(
        chest("strata:iron_ingot").as_deref(),
        Some("strata:iron_plate_chestplate")
    );
    assert_eq!(
        chest("strata:leather").as_deref(),
        Some("strata:leather_tunic")
    );
    assert_eq!(
        chest("strata:diamond_scales").as_deref(),
        Some("strata:diamond_scale_chestplate")
    );
}

#[test]
fn every_tool_weapon_and_armour_piece_can_be_made() {
    let content = game();
    let made = content.items.iter().filter(|(_, _, item)| {
        item.tool.is_some() || item.gear.weapon.is_some() || item.gear.armor.is_some()
    });
    let mut count = 0;
    for (key, name, _) in made {
        assert!(content.recipes.making(key).next().is_some(), "{name}");
        count += 1;
    }
    assert!(count >= 60, "{count}");
}

#[test]
fn armour_weighs_by_its_kind_and_metals_come_in_several_weights() {
    let content = game();
    let armor = |name: &str| {
        let item = content.items.get(item(&content, name)).unwrap();
        item.gear.armor.clone().unwrap()
    };
    let weight = |name: &str| armor(name).weight.name().to_owned();
    assert_eq!(weight("strata:leather_tunic"), "light");
    assert_eq!(weight("strata:cloth_tunic"), "light");
    assert_eq!(weight("strata:iron_mail_shirt"), "medium");
    assert_eq!(weight("strata:diamond_scale_chestplate"), "medium");
    assert_eq!(weight("strata:iron_plate_chestplate"), "heavy");
    assert_eq!(weight("strata:gold_plate_chestplate"), "heavy");
    assert!(armor("strata:iron_plate_chestplate").burden.speed > 0.0);
    assert_eq!(armor("strata:iron_mail_shirt").burden.speed, 0.0);
    assert!(armor("strata:iron_mail_shirt").burden.stamina > 0.0);
    let item_of = |name: &str| content.items.get(item(&content, name)).unwrap();
    assert_eq!(
        item_of("strata:iron_plate_chestplate").durability,
        Some(15 * 16)
    );
    assert_eq!(
        item_of("strata:diamond_plate_boots").durability,
        Some(33 * 13)
    );
    assert_eq!(
        item_of("strata:leather_cap").gear.upgrade_slots,
        vec![id("strata:craft")]
    );
    let protection = |name: &str| armor(name).protection;
    assert!(protection("strata:iron_plate_chestplate") > protection("strata:iron_mail_shirt"));
    assert!(protection("strata:iron_mail_shirt") > protection("strata:leather_tunic"));
}

#[test]
fn tooltips_tell_damage_protection_and_upgrades() {
    let content = game();
    let tooltip = |name: &str, lang| content.item_tooltip(item(&content, name), lang);
    let sword = tooltip("strata:iron_sword", "fr");
    assert!(sword.contains("Dégâts : 6"), "{sword}");
    assert!(sword.contains("amélioration : artisanal ×1"), "{sword}");
    let plate = tooltip("strata:diamond_plate_chestplate", "en");
    assert!(plate.contains("Protection: 8 (heavy armour)"), "{plate}");
    let multi = tooltip("strata:multi_tool", "en");
    assert!(multi.contains("pickaxe, axe, shovel"), "{multi}");
    let sickle = tooltip("strata:sickle", "fr");
    assert!(sickle.contains("2 blocs"), "{sickle}");
}
