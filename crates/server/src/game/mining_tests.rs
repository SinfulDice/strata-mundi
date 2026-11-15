#![allow(clippy::unwrap_used)]

use super::mining::long_enough;
use super::tests::{World, block, hold, mine, wait, world_with};
use super::*;
use strata_core::content::{Content, Item, ItemId, ItemKind, TextureId, ToolDef};
use strata_core::content::{LootDrop, LootEntry, LootPool, LootTable};
use strata_protocol::{ActionId, WorldAction};

/// Beside the player, on the stone floor.
const SPOT: BlockPos = BlockPos::new(10, 64, 8);

fn survival_with(name: &str) -> World {
    let mut w = world_with(add_pickaxe);
    w.game.set_mode(GameMode::Survival);
    assert!(w.loader.set_block(SPOT, block(&w.content, name)));
    w.game.take_messages();
    w
}

/// A stone-level test pickaxe that breaks after 10 uses.
pub(super) fn add_pickaxe(content: &mut Content) {
    let item = Item {
        max_stack: 1,
        category: None,
        kind: ItemKind::Flat(TextureId::MISSING),
        action: None,
        durability: Some(10),
        equip: None,
        tool: Some(ToolDef {
            kinds: vec!["strata:pickaxe".parse().unwrap()],
            level: 1,
            speed: 4.0,
            sweep: 0,
        }),
        remainder: None,
        kit: None,
        module: None,
        gear: Default::default(),
    };
    content
        .items
        .insert("test:pickaxe".parse().unwrap(), item)
        .unwrap();
}

fn item(w: &World, name: &str) -> ItemId {
    w.content.items.key(&name.parse().unwrap()).unwrap()
}

fn dropped(w: &World) -> Vec<(ItemId, u16)> {
    w.game
        .ecs
        .drops
        .iter()
        .map(|(_, d)| (d.stack.item, d.stack.count))
        .collect()
}

fn cracks(w: &mut World) -> Vec<Option<u8>> {
    w.game
        .take_messages()
        .into_iter()
        .filter_map(|message| match message {
            ServerMessage::Cracking { progress, .. } => Some(progress),
            _ => None,
        })
        .collect()
}

#[test]
fn the_slack_is_two_ticks_or_a_tenth() {
    assert!(long_enough(13, 15) && !long_enough(12, 15));
    assert!(long_enough(135, 150) && !long_enough(134, 150));
    assert!(long_enough(0, 2) && long_enough(0, 0));
}

#[test]
fn survival_breaks_only_once_hit_long_enough() {
    let mut w = survival_with("strata:dirt");
    hold(&mut w, None);
    assert!(
        w.game.break_block(&mut w.loader, SPOT).is_empty(),
        "never hit"
    );
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 12);
    assert!(
        w.game.break_block(&mut w.loader, SPOT).is_empty(),
        "dirt takes 15 ticks by hand"
    );
    wait(&mut w, 1);
    assert!(!w.game.break_block(&mut w.loader, SPOT).is_empty());
    assert_eq!(dropped(&w), vec![(item(&w, "strata:dirt"), 1)]);
}

#[test]
fn instant_blocks_need_no_start() {
    let mut w = survival_with("strata:poppy");
    hold(&mut w, None);
    assert!(!w.game.break_block(&mut w.loader, SPOT).is_empty());
    assert_eq!(dropped(&w), vec![(item(&w, "strata:poppy"), 1)]);
}

#[test]
fn creative_breaks_at_once_and_drops_nothing() {
    let mut w = survival_with("strata:obsidian");
    w.game.set_mode(GameMode::Creative);
    assert!(!w.game.break_block(&mut w.loader, SPOT).is_empty());
    assert!(dropped(&w).is_empty());
}

#[test]
fn changing_the_held_item_or_the_block_ends_the_break() {
    let mut w = survival_with("strata:dirt");
    hold(&mut w, None);
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 8);
    let stone = item(&w, "strata:stone");
    hold(&mut w, Some(stone));
    wait(&mut w, 8);
    assert!(w.game.break_block(&mut w.loader, SPOT).is_empty());
    assert_eq!(cracks(&mut w).last(), Some(&None), "the cracks went away");
    hold(&mut w, None);
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 8);
    assert!(w.loader.set_block(SPOT, block(&w.content, "strata:sand")));
    wait(&mut w, 8);
    assert!(w.game.break_block(&mut w.loader, SPOT).is_empty());
}

#[test]
fn stone_by_hand_is_slow_and_gives_nothing() {
    let mut w = survival_with("strata:stone");
    hold(&mut w, None);
    let stone = block(&w.content, "strata:stone");
    assert_eq!(w.game.rules.mining.break_ticks(stone, None), Some(150));
    assert!(!mine(&mut w, SPOT).is_empty());
    assert!(dropped(&w).is_empty());
}

#[test]
fn the_right_pickaxe_is_fast_harvests_and_wears() {
    let mut w = survival_with("strata:stone");
    let pickaxe = item(&w, "test:pickaxe");
    hold(&mut w, Some(pickaxe));
    let stone = block(&w.content, "strata:stone");
    assert_eq!(
        w.game.rules.mining.break_ticks(stone, Some(pickaxe)),
        Some(12)
    );
    assert!(!mine(&mut w, SPOT).is_empty());
    assert_eq!(dropped(&w), vec![(item(&w, "strata:cobblestone"), 1)]);
    let inventory = w.game.inventory();
    assert_eq!(inventory.selected_stack().unwrap().durability, Some(9));
}

#[test]
fn a_pickaxe_too_weak_for_the_ore_gives_nothing() {
    let mut w = survival_with("strata:diamond_ore");
    let pickaxe = item(&w, "test:pickaxe");
    hold(&mut w, Some(pickaxe));
    assert!(!mine(&mut w, SPOT).is_empty());
    assert!(dropped(&w).is_empty());
}

#[test]
fn grass_gives_dirt_and_leaves_give_nothing_by_hand() {
    let mut w = survival_with("strata:grass_block");
    hold(&mut w, None);
    assert!(!mine(&mut w, SPOT).is_empty());
    assert_eq!(dropped(&w), vec![(item(&w, "strata:dirt"), 1)]);
    let mut w = survival_with("strata:oak_leaves");
    hold(&mut w, None);
    assert!(!mine(&mut w, SPOT).is_empty());
    assert!(dropped(&w).is_empty());
}

#[test]
fn cracks_grow_stage_by_stage_then_go() {
    let mut w = survival_with("strata:oak_planks");
    hold(&mut w, None);
    assert!(!mine(&mut w, SPOT).is_empty());
    let reports = cracks(&mut w);
    let (last, grown) = reports.split_last().unwrap();
    assert_eq!(last, &None);
    assert_eq!(grown.len(), w.content.crack_stages.len(), "one per stage");
    assert!(grown.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn starting_a_break_skips_the_rate_limit_but_keeps_the_order() {
    let mut w = survival_with("strata:dirt");
    hold(&mut w, None);
    let instant = BlockPos::new(10, 64, 9);
    assert!(
        w.loader
            .set_block(instant, block(&w.content, "strata:poppy"))
    );
    let poppy = WorldAction::Break { pos: instant };
    assert!(
        !w.game
            .request_action(&mut w.loader, ActionId(1), poppy)
            .is_empty()
    );
    let start = WorldAction::StartBreak { pos: SPOT };
    w.game.request_action(&mut w.loader, ActionId(2), start);
    let mining = w.game.ecs.controls.get(w.game.player).unwrap().mining;
    assert_eq!(mining.map(|m| m.pos), Some(SPOT), "started at once");
    wait(&mut w, 13);
    let action = WorldAction::Break { pos: SPOT };
    assert!(
        !w.game
            .request_action(&mut w.loader, ActionId(3), action)
            .is_empty()
    );
}

#[test]
fn the_server_breaks_the_block_when_the_client_is_late() {
    let mut w = survival_with("strata:dirt");
    hold(&mut w, None);
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 5);
    assert!(
        w.game.break_block(&mut w.loader, SPOT).is_empty(),
        "too early"
    );
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 10);
    assert_eq!(
        w.loader.block(SPOT),
        Some(BlockId::AIR),
        "broken at 15 ticks"
    );
    assert_eq!(dropped(&w), vec![(item(&w, "strata:dirt"), 1)]);
    assert!(
        w.game.break_block(&mut w.loader, SPOT).is_empty(),
        "late Break"
    );
}

#[test]
fn going_out_of_reach_or_into_creative_ends_the_break() {
    let mut w = survival_with("strata:oak_planks");
    hold(&mut w, None);
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 10);
    w.game.ecs.bodies.get_mut(w.game.player).unwrap().position.x = 40.0;
    wait(&mut w, 1);
    assert!(mining(&w).is_none());
    assert_eq!(cracks(&mut w).last(), Some(&None));
    w.game.ecs.bodies.get_mut(w.game.player).unwrap().position.x = 8.5;
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 10);
    w.game.set_mode(GameMode::Creative);
    wait(&mut w, 1);
    assert!(mining(&w).is_none());
    assert_eq!(cracks(&mut w).last(), Some(&None));
}

#[test]
fn stopping_clears_the_cracks_and_a_new_start_keeps_the_time() {
    let mut w = survival_with("strata:oak_planks");
    hold(&mut w, None);
    w.game.start_break(&w.loader, SPOT);
    wait(&mut w, 10);
    w.game.start_break(&w.loader, SPOT);
    assert_eq!(mining(&w).unwrap().started, 0, "same block, same item");
    w.game.take_messages();
    let stop = WorldAction::StopBreak;
    w.game.request_action(&mut w.loader, ActionId(1), stop);
    assert_eq!(cracks(&mut w), vec![None]);
    assert!(mining(&w).is_none());
}

#[test]
fn breaks_are_limited_to_one_every_two_ticks() {
    let mut w = survival_with("strata:poppy");
    hold(&mut w, None);
    let other = BlockPos::new(10, 64, 9);
    assert!(w.loader.set_block(other, block(&w.content, "strata:poppy")));
    assert!(!w.game.break_block(&mut w.loader, SPOT).is_empty());
    assert!(w.game.break_block(&mut w.loader, other).is_empty());
    wait(&mut w, 2);
    assert!(!w.game.break_block(&mut w.loader, other).is_empty());
}

#[test]
fn loot_is_dropped_in_full_stacks() {
    let mut w = world_with(|content| {
        let bucket = content
            .items
            .key(&"strata:bucket".parse().unwrap())
            .unwrap();
        let table = LootTable {
            pools: vec![LootPool {
                rolls: 1,
                conditions: Vec::new(),
                entries: vec![LootEntry {
                    drop: LootDrop::Item(bucket),
                    weight: 1,
                    count: (20, 20),
                    conditions: Vec::new(),
                }],
            }],
        };
        let name: strata_core::id::Identifier = "test:buckets".parse().unwrap();
        content.loot_tables.insert(name.clone(), table);
        let poppy = content
            .blocks
            .key(&"strata:poppy".parse().unwrap())
            .unwrap();
        content.blocks.get_mut(poppy).unwrap().loot = Some(name);
    });
    w.game.set_mode(GameMode::Survival);
    assert!(w.loader.set_block(SPOT, block(&w.content, "strata:poppy")));
    hold(&mut w, None);
    assert!(!w.game.break_block(&mut w.loader, SPOT).is_empty());
    let mut counts: Vec<u16> = dropped(&w).iter().map(|(_, count)| *count).collect();
    counts.sort_unstable();
    assert_eq!(counts, vec![4, 16]);
}

fn mining(w: &World) -> Option<crate::ecs::Mining> {
    w.game.ecs.controls.get(w.game.player).unwrap().mining
}
