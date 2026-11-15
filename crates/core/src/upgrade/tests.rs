#![allow(clippy::unwrap_used)]

use super::installed::install;
use super::*;
use crate::content::{Content, Face};
use crate::coords::BlockPos;
use crate::inventory::{Click, CraftGrid, ItemStack, PlayerInventory, Window};
use crate::pack::PackSet;
use std::path::Path;
use std::sync::OnceLock;

fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        let (content, errors) = Content::load(&packs, &PackSet::default());
        assert!(errors.is_empty(), "{errors:#?}");
        content
    })
}

fn id(name: &str) -> Identifier {
    name.parse().unwrap()
}

fn stack(name: &str) -> ItemStack {
    let items = &content().items;
    ItemStack::new(items.key(&id(name)).unwrap(), 1, items)
}

fn some(name: &str, count: u16) -> ItemStack {
    ItemStack {
        count,
        ..stack(name)
    }
}

fn stats(stack: &ItemStack) -> ItemStats {
    stack_stats(stack, &content().items, &content().upgrades)
}

fn index(name: &str) -> u16 {
    content().upgrades.index_of(&id(name)).unwrap()
}

/// The anvil's result for `item` and `material`, `choice` an upgrade name.
fn outcome(item: &ItemStack, material: &ItemStack, choice: Option<&str>) -> Option<Outcome> {
    let c = content();
    anvil_outcome(
        Some(item),
        Some(material),
        choice.map(index),
        &c.items,
        &c.upgrades,
    )
}

fn upgraded(item: &str, upgrades: &[(&str, u8)]) -> ItemStack {
    let mut stack = stack(item);
    for (name, level) in upgrades {
        let held = installed(&stack);
        install(&mut stack.data, &held, &id(name), *level);
    }
    stack
}

#[test]
fn sharpening_raises_damage_and_mining_speed() {
    let plain = stats(&stack("strata:iron_sword"));
    let sharp = stats(&upgraded("strata:iron_sword", &[("strata:sharpening", 2)]));
    let damage = |s: &ItemStats| s.weapon.unwrap().damage;
    assert!((damage(&sharp) - damage(&plain) * 1.2).abs() < 1e-4);
    let pick = stats(&upgraded(
        "strata:iron_pickaxe",
        &[("strata:sharpening", 1)],
    ));
    let speed = pick.tool.unwrap().speed;
    assert!((speed - 6.6).abs() < 1e-4, "{speed}");
}

#[test]
fn reinforcement_and_lightening_change_armour_and_uses() {
    let plain = stats(&stack("strata:iron_plate_chestplate"));
    let strong = stats(&upgraded(
        "strata:iron_plate_chestplate",
        &[("strata:reinforcement", 1)],
    ));
    assert_eq!(strong.armor.as_ref().unwrap().protection, 7);
    assert_eq!(strong.durability, Some(288), "240 + 20 %");
    let light = stats(&upgraded(
        "strata:iron_plate_chestplate",
        &[("strata:lightening", 2)],
    ));
    let speed = |s: &ItemStats| s.armor.as_ref().unwrap().burden.speed;
    assert!((speed(&light) - speed(&plain) / 2.0).abs() < 1e-6);
}

#[test]
fn the_same_effect_does_not_add_up() {
    let material = crate::crafting::Ingredient::item(stack("strata:stick").item);
    let level = |percent| UpgradeLevel {
        effects: vec![Effect::Stat {
            stat: id("strata:damage"),
            op: StatOp::Percent,
            value: percent,
        }],
        xp: 0,
        material: material.clone(),
        count: 1,
    };
    let upgrade = |name: &str, percent| Upgrade {
        id: id(name),
        slot: id("strata:craft"),
        applies_to: None,
        levels: vec![level(percent)],
        requires: Vec::new(),
        excludes: Vec::new(),
    };
    let both = Upgrades::new(
        vec![upgrade("test:keen", 10.0), upgrade("test:keener", 30.0)],
        Default::default(),
        AnvilRules::default(),
    );
    let mut sword = stack("strata:iron_sword");
    install(&mut sword.data, &[], &id("test:keen"), 1);
    let held = installed(&sword);
    install(&mut sword.data, &held, &id("test:keener"), 1);
    let base = content()
        .items
        .get(sword.item)
        .unwrap()
        .gear
        .weapon
        .unwrap();
    let total = stack_stats(&sword, &content().items, &both).weapon.unwrap();
    assert!(
        (total.damage - base.damage * 1.3).abs() < 1e-4,
        "the strongest only"
    );
}

#[test]
fn an_item_offers_the_upgrades_that_suit_it() {
    let c = content();
    let names = |stack: &ItemStack| -> Vec<String> {
        choices(stack, &c.items, &c.upgrades)
            .iter()
            .map(|choice| choice.upgrade.id.to_string())
            .collect()
    };
    let sword = names(&stack("strata:iron_sword"));
    assert_eq!(sword, ["strata:reinforcement", "strata:sharpening"]);
    let armour = names(&stack("strata:leather_tunic"));
    assert_eq!(armour, ["strata:lightening", "strata:reinforcement"]);
    assert!(names(&stack("strata:wooden_pickaxe")).is_empty(), "no slot");
    let hammer = names(&stack("strata:iron_hammer"));
    assert!(hammer.contains(&"strata:hammer_area".to_owned()));
}

#[test]
fn slots_exclusions_and_last_levels_are_respected() {
    let c = content();
    let sharp = upgraded("strata:iron_sword", &[("strata:sharpening", 1)]);
    let offered = choices(&sharp, &c.items, &c.upgrades);
    let find = |name: &str| offered.iter().find(|o| o.upgrade.id == id(name)).cloned();
    assert_eq!(find("strata:sharpening").unwrap().level, 2);
    assert_eq!(find("strata:sharpening").unwrap().blocked, None);
    let reinforce = find("strata:reinforcement").unwrap();
    assert_eq!(
        reinforce.blocked,
        Some(Blocked::NoSlot),
        "one craft slot in iron"
    );
    let gold = upgraded("strata:gold_plate_helmet", &[("strata:lightening", 1)]);
    let offered = choices(&gold, &c.items, &c.upgrades);
    let reinforce = offered
        .iter()
        .find(|o| o.upgrade.id == id("strata:reinforcement"))
        .unwrap();
    let excluded = Some(Blocked::Excludes(id("strata:lightening")));
    assert_eq!(reinforce.blocked, excluded);
    let top = upgraded("strata:iron_sword", &[("strata:sharpening", 3)]);
    let offered = choices(&top, &c.items, &c.upgrades);
    assert!(
        offered
            .iter()
            .all(|o| o.upgrade.id != id("strata:sharpening"))
    );
}

#[test]
fn an_upgrade_costs_its_level_materials_and_experience() {
    let sword = stack("strata:iron_sword");
    let short = some("strata:copper_ingot", 1);
    assert_eq!(outcome(&sword, &short, Some("strata:sharpening")), None);
    let wrong = some("strata:iron_ingot", 5);
    assert_eq!(outcome(&sword, &wrong, Some("strata:sharpening")), None);
    let paid = some("strata:copper_ingot", 3);
    let done = outcome(&sword, &paid, Some("strata:sharpening")).unwrap();
    assert_eq!((done.material, done.xp), (2, 2));
    let held = installed(&done.result);
    assert_eq!(held, [(id("strata:sharpening"), 1)]);
    let next = outcome(
        &done.result,
        &some("strata:iron_ingot", 2),
        Some("strata:sharpening"),
    );
    let next = next.unwrap();
    assert_eq!((next.material, next.xp), (2, 4));
    assert_eq!(installed(&next.result), [(id("strata:sharpening"), 2)]);
}

#[test]
fn reinforcing_gives_the_new_uses_at_once() {
    let mut pick = stack("strata:iron_pickaxe");
    pick.durability = Some(100);
    let ingots = some("strata:iron_ingot", 2);
    let done = outcome(&pick, &ingots, Some("strata:reinforcement")).unwrap();
    assert_eq!(done.result.durability, Some(150), "250 → 300 uses: 50 more");
}

#[test]
fn the_tier_material_repairs_a_quarter_each() {
    let mut pick = stack("strata:iron_pickaxe");
    pick.durability = Some(50);
    let done = outcome(&pick, &some("strata:iron_ingot", 10), None).unwrap();
    assert_eq!(done.result.durability, Some(250));
    assert_eq!((done.material, done.xp), (4, 1));
    let mut again = done.result.clone();
    again.durability = Some(200);
    let second = outcome(&again, &some("strata:iron_ingot", 10), None).unwrap();
    assert_eq!(
        (second.material, second.xp),
        (1, 2),
        "each repair costs more"
    );
    assert_eq!(repairs(&second.result), 2);
    assert_eq!(outcome(&pick, &some("strata:gold_ingot", 4), None), None);
    let full = stack("strata:iron_pickaxe");
    assert_eq!(outcome(&full, &some("strata:iron_ingot", 4), None), None);
}

#[test]
fn two_like_items_join_their_uses() {
    let mut a = stack("strata:iron_sword");
    a.durability = Some(100);
    let mut b = stack("strata:iron_sword");
    b.durability = Some(80);
    let done = outcome(&a, &b, None).unwrap();
    assert_eq!(done.result.durability, Some(210), "100 + 80 + 12 % of 250");
    assert_eq!(done.material, 1);
    let upgraded = upgraded("strata:iron_sword", &[("strata:sharpening", 1)]);
    assert_eq!(
        outcome(&a, &upgraded, None),
        None,
        "an upgraded one is kept"
    );
}

#[test]
fn a_broken_item_kept_is_repaired() {
    let mut boots = stack("strata:leather_boots");
    boots.durability = Some(0);
    let done = outcome(&boots, &some("strata:leather", 1), None).unwrap();
    assert!(done.result.durability.unwrap() > 0);
    assert!(!done.result.is_broken());
}

#[test]
fn hammer_areas_unlock_one_after_the_other() {
    let plain = stats(&stack("strata:iron_hammer"));
    assert!(plain.areas.is_empty());
    let hammer = upgraded("strata:iron_hammer", &[("strata:hammer_area", 3)]);
    let areas = stats(&hammer).areas;
    let sizes: Vec<(u8, u8)> = areas.iter().map(|a| (a.width, a.height)).collect();
    assert_eq!(sizes, [(1, 1), (3, 1), (5, 1), (3, 3)]);
    assert_eq!(
        chosen_area(&hammer, &areas),
        areas[3],
        "the largest at first"
    );
    let smaller = with_area(&hammer, 1);
    assert_eq!(chosen_area(&smaller, &areas), areas[1]);
    let costs: Vec<u32> = content()
        .upgrades
        .get(&id("strata:hammer_area"))
        .unwrap()
        .levels
        .iter()
        .map(|level| level.xp)
        .collect();
    assert!(costs.windows(2).all(|pair| pair[0] < pair[1]), "{costs:?}");
}

#[test]
fn an_area_lies_across_the_face_hit() {
    let center = BlockPos::new(0, 10, 0);
    let wide = AreaSize {
        width: 3,
        height: 1,
    };
    let wall = area_cells(center, Face::North, wide, 0.0);
    assert_eq!(wall[0], center);
    assert!(wall.iter().all(|p| p.y == 10 && p.z == 0) && wall.len() == 3);
    let side = area_cells(center, Face::East, wide, 0.0);
    assert!(side.iter().all(|p| p.x == 0 && p.y == 10));
    let tall = AreaSize {
        width: 3,
        height: 5,
    };
    let cells = area_cells(center, Face::South, tall, 0.0);
    let ys: Vec<i32> = cells.iter().map(|p| p.y).collect();
    assert_eq!((ys.iter().min(), ys.iter().max()), (Some(&8), Some(&12)));
    let ahead = AreaSize {
        width: 1,
        height: 3,
    };
    let floor_north = area_cells(center, Face::Up, ahead, 0.0);
    assert!(floor_north.iter().all(|p| p.x == 0 && p.y == 10));
    let floor_east = area_cells(center, Face::Up, ahead, std::f32::consts::FRAC_PI_2);
    assert!(floor_east.iter().all(|p| p.z == 0 && p.y == 10));
}

#[test]
fn taking_the_result_uses_the_item_and_the_material_paid() {
    let mut player = PlayerInventory::default();
    let mut grid = CraftGrid::anvil();
    grid.slots.slots[ANVIL_ITEM] = Some(stack("strata:iron_sword"));
    grid.slots.slots[ANVIL_MATERIAL] = Some(some("strata:copper_ingot", 5));
    let done = outcome(
        grid.slots.get(ANVIL_ITEM).unwrap(),
        grid.slots.get(ANVIL_MATERIAL).unwrap(),
        Some("strata:sharpening"),
    )
    .unwrap();
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    let items = &content().items;
    assert!(take_outcome(&mut window, done, Click::Primary, items));
    assert!(grid.slots.get(ANVIL_ITEM).is_none());
    assert_eq!(grid.slots.get(ANVIL_MATERIAL).unwrap().count, 3);
    let carried = player.cursor.unwrap();
    assert_eq!(installed(&carried).len(), 1);
}

#[test]
fn a_worn_or_upgraded_material_is_never_used_up() {
    let sword = stack("strata:iron_sword");
    let mut copper = some("strata:copper_ingot", 3);
    copper.data = upgraded("strata:iron_sword", &[("strata:sharpening", 1)]).data;
    assert_eq!(outcome(&sword, &copper, Some("strata:sharpening")), None);
    let mut worn = stack("strata:iron_pickaxe");
    worn.durability = Some(10);
    let mut ingots = some("strata:iron_ingot", 3);
    ingots.durability = Some(1);
    assert_eq!(outcome(&worn, &ingots, None), None);
}

#[test]
fn unknown_stats_and_links_are_reported() {
    let mut errors = Vec::new();
    let effect = Effect::Stat {
        stat: id("test:luck"),
        op: StatOp::Add,
        value: 1.0,
    };
    let stats = &content().upgrades.stats;
    let source = Path::new("upgrades/test.ron");
    assert_eq!(
        super::load::bounded(effect, stats, source, &mut errors),
        None
    );
    let huge = Effect::Stat {
        stat: id("strata:damage"),
        op: StatOp::Percent,
        value: 1.0e9,
    };
    let kept = super::load::bounded(huge, stats, source, &mut errors).unwrap();
    assert!(matches!(kept, Effect::Stat { value, .. } if value <= 1000.0));
    let mut lonely = content()
        .upgrades
        .get(&id("strata:lightening"))
        .unwrap()
        .clone();
    lonely.requires = vec![id("test:nothing")];
    lonely.excludes.clear();
    super::load::check_links(&[lonely], &mut errors);
    assert_eq!(
        errors.len(),
        2,
        "an unknown stat and an unknown upgrade: {errors:?}"
    );
}

#[test]
fn a_tooltip_lists_only_the_bricks_that_change_the_item() {
    let c = content();
    let pick = upgraded("strata:iron_pickaxe", &[("strata:sharpening", 1)]);
    let lines = c.stack_upgrade_lines(&pick, "en").join("\n");
    assert!(
        lines.contains("mining speed") && !lines.contains("damage"),
        "{lines}"
    );
}
