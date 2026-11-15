#![allow(clippy::unwrap_used)]

use super::tests::{World, block, hold, mine, wait, world};
use super::*;
use strata_core::content::{Face, ItemId};
use strata_core::inventory::{ARMOR, ItemStack};

const SPOT: BlockPos = BlockPos::new(10, 64, 8);

fn survival() -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    w.game.take_messages();
    w
}

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn set(w: &mut World, pos: BlockPos, name: &str) {
    assert!(w.loader.set_block(pos, block(&w.content, name)));
}

fn dropped(w: &World, name: &str) -> u16 {
    let wanted = item(w, name);
    w.game
        .ecs
        .drops
        .iter()
        .filter(|(_, d)| d.stack.item == wanted)
        .map(|(_, d)| d.stack.count)
        .sum()
}

fn held_uses(w: &World) -> Option<u16> {
    let inventory = w.game.ecs.inventories.get(w.game.player)?;
    inventory.selected_stack()?.durability
}

#[test]
fn diamond_ore_needs_an_iron_pickaxe_and_obsidian_a_diamond_one() {
    let mut w = survival();
    let ore = block(&w.content, "strata:diamond_ore");
    let obsidian = block(&w.content, "strata:obsidian");
    let mining = std::sync::Arc::clone(&w.game.rules.mining);
    let harvests = |w: &World, block, pickaxe: &str| {
        let tool = mining.tool(Some(item(w, pickaxe))).cloned();
        w.content.blocks.get(block).unwrap().harvests(tool.as_ref())
    };
    assert!(!harvests(&w, ore, "strata:stone_pickaxe"));
    assert!(!harvests(&w, ore, "strata:wooden_pickaxe"));
    assert!(harvests(&w, ore, "strata:iron_pickaxe"));
    assert!(harvests(&w, ore, "strata:golden_pickaxe"));
    assert!(!harvests(&w, obsidian, "strata:iron_pickaxe"));
    assert!(harvests(&w, obsidian, "strata:diamond_pickaxe"));
    set(&mut w, SPOT, "strata:diamond_ore");
    let pickaxe = item(&w, "strata:iron_pickaxe");
    hold(&mut w, Some(pickaxe));
    mine(&mut w, SPOT);
    assert_eq!(dropped(&w, "strata:diamond"), 1);
    assert_eq!(held_uses(&w), Some(249), "the tier's 250 uses, one spent");
}

#[test]
fn faster_tiers_break_sooner() {
    let w = survival();
    let stone = block(&w.content, "strata:stone");
    let ticks = |name: &str| {
        let mining = &w.game.rules.mining;
        mining
            .break_ticks(stone, mining.tool(Some(item(&w, name))))
            .unwrap()
    };
    let order = [
        ticks("strata:golden_pickaxe"),
        ticks("strata:diamond_pickaxe"),
        ticks("strata:iron_pickaxe"),
        ticks("strata:stone_pickaxe"),
        ticks("strata:wooden_pickaxe"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

#[test]
fn the_multi_tool_digs_chops_and_shovels() {
    let w = survival();
    let rules = &w.game.rules.mining;
    let multi = rules.tool(Some(item(&w, "strata:multi_tool")));
    for (name, single) in [
        ("strata:stone", "strata:iron_pickaxe"),
        ("strata:oak_log", "strata:iron_axe"),
        ("strata:dirt", "strata:iron_shovel"),
    ] {
        let target = block(&w.content, name);
        let alone = rules.break_ticks(target, rules.tool(Some(item(&w, single))));
        assert_eq!(rules.break_ticks(target, multi), alone, "{name}");
        assert!(alone < rules.break_ticks(target, None), "{name}");
    }
    let durability = |name: &str| w.content.items.get(item(&w, name)).unwrap().durability;
    assert!(durability("strata:multi_tool") < durability("strata:iron_pickaxe"));
}

#[test]
fn the_sickle_cuts_the_plants_around_for_one_use() {
    let mut w = survival();
    for dx in -2..=2 {
        for dz in -2..=2 {
            set(
                &mut w,
                BlockPos::new(10 + dx, 64, 8 + dz),
                "strata:tall_grass",
            );
        }
    }
    let far = BlockPos::new(13, 64, 8);
    set(&mut w, far, "strata:tall_grass");
    let sickle = item(&w, "strata:sickle");
    hold(&mut w, Some(sickle));
    let full = held_uses(&w).unwrap();
    assert!(!w.game.break_block(&mut w.loader, SPOT).is_empty());
    let air = |w: &World, pos| w.loader.block(pos) == Some(BlockId::AIR);
    assert!(air(&w, BlockPos::new(8, 64, 6)) && air(&w, BlockPos::new(12, 64, 10)));
    assert!(!air(&w, far), "three blocks away stays");
    assert_eq!(held_uses(&w), Some(full - 1));
    let fiber = item(&w, "strata:plant_fiber");
    let heaps = w
        .game
        .ecs
        .drops
        .iter()
        .filter(|(_, d)| d.stack.item == fiber)
        .count();
    assert!(
        dropped(&w, "strata:plant_fiber") > 1,
        "24 plants, one chance in four each"
    );
    assert!(
        heaps <= 2,
        "the cut plant's fibre and one heap for the others: {heaps}"
    );
    wait(&mut w, 2);
    hold(&mut w, None);
    set(&mut w, SPOT, "strata:tall_grass");
    set(&mut w, BlockPos::new(11, 64, 8), "strata:tall_grass");
    assert!(!w.game.break_block(&mut w.loader, SPOT).is_empty());
    assert!(!air(&w, BlockPos::new(11, 64, 8)), "a hand cuts one plant");
}

#[test]
fn a_worn_out_tool_breaks_or_stays_broken_by_the_world_option() {
    let mut w = survival();
    let pickaxe = item(&w, "strata:wooden_pickaxe");
    let mut worn = ItemStack::new(pickaxe, 1, &w.game.rules);
    worn.durability = Some(1);
    let put = |w: &mut World, stack: &ItemStack| {
        let inventory = w.game.ecs.inventories.get_mut(w.game.player).unwrap();
        *inventory.selected_slot_mut() = Some(stack.clone());
    };
    put(&mut w, &worn);
    w.game.wear_held(1);
    assert!(held_uses(&w).is_none(), "broken and gone");
    w.game.durability = DurabilityMode::KeepBroken;
    put(&mut w, &worn);
    w.game.wear_held(1);
    assert_eq!(held_uses(&w), Some(0), "kept, broken");
    assert_eq!(w.game.held_item(), None, "a broken tool is a bare hand");
    set(&mut w, SPOT, "strata:stone");
    let stone = block(&w.content, "strata:stone");
    let ticks = w
        .game
        .rules
        .mining
        .break_ticks(stone, w.game.held_tool().as_ref());
    assert_eq!(ticks, Some(150));
}

#[test]
fn a_hoe_tills_open_dirt_and_a_chisel_carves_in_a_cycle() {
    let mut w = survival();
    set(&mut w, SPOT, "strata:dirt");
    let hoe = item(&w, "strata:wooden_hoe");
    hold(&mut w, Some(hoe));
    assert!(
        !w.game
            .use_item(&mut w.loader, hoe, SPOT, Face::Up)
            .is_empty()
    );
    assert_eq!(
        w.loader.block(SPOT),
        Some(block(&w.content, "strata:farmland"))
    );
    assert_eq!(held_uses(&w), Some(58));
    let buried = BlockPos::new(10, 63, 8);
    set(&mut w, buried, "strata:dirt");
    set(&mut w, SPOT, "strata:stone");
    wait(&mut w, 2);
    assert!(
        w.game
            .use_item(&mut w.loader, hoe, buried, Face::Up)
            .is_empty()
    );
    let chisel = item(&w, "strata:chisel");
    hold(&mut w, Some(chisel));
    set(&mut w, SPOT, "strata:stone_bricks");
    let mut seen = Vec::new();
    for _ in 0..3 {
        wait(&mut w, 2);
        assert!(
            !w.game
                .use_item(&mut w.loader, chisel, SPOT, Face::Up)
                .is_empty()
        );
        seen.push(w.loader.block(SPOT).unwrap());
    }
    let names = [
        "strata:chiseled_stone_bricks",
        "strata:patterned_stone_bricks",
        "strata:stone_bricks",
    ];
    let expected: Vec<BlockId> = names.iter().map(|name| block(&w.content, name)).collect();
    assert_eq!(seen, expected);
}

#[test]
fn heavy_armour_slows_the_player_light_armour_does_not() {
    let mut w = survival();
    let wear = |w: &mut World, names: &[&str]| {
        let stacks: Vec<_> = names
            .iter()
            .map(|name| Some(ItemStack::new(item(w, name), 1, &w.game.rules)))
            .collect();
        let inventory = w.game.ecs.inventories.get_mut(w.game.player).unwrap();
        inventory.slots.slots[ARMOR].clone_from_slice(&stacks);
        wait(w, 1);
        w.game.movement().unwrap().speed
    };
    let leather = [
        "strata:leather_cap",
        "strata:leather_tunic",
        "strata:leather_pants",
        "strata:leather_boots",
    ];
    assert_eq!(wear(&mut w, &leather), 1.0);
    let plate = [
        "strata:iron_plate_helmet",
        "strata:iron_plate_chestplate",
        "strata:iron_plate_leggings",
        "strata:iron_plate_boots",
    ];
    assert!((wear(&mut w, &plate) - 0.9).abs() < 1e-6);
    let items = w.game.rules.items();
    let inventory = w.game.ecs.inventories.get(w.game.player).unwrap();
    let worn = ARMOR.filter_map(|slot| inventory.slots.get(slot));
    let upgrades = &w.game.rules.upgrades;
    assert_eq!(
        strata_core::gear::worn_protection(worn, items, upgrades),
        15
    );
}
