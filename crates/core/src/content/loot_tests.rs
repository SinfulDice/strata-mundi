use super::*;

fn kind(name: &str) -> Identifier {
    Identifier::new("strata", name).unwrap()
}

fn entry(drop: LootDrop, weight: u32, count: (u16, u16)) -> LootEntry {
    LootEntry {
        drop,
        weight,
        count,
        conditions: Vec::new(),
    }
}

fn table(entries: Vec<LootEntry>) -> LootTable {
    LootTable {
        pools: vec![LootPool {
            rolls: 1,
            conditions: Vec::new(),
            entries,
        }],
    }
}

fn bare(itself: Option<ItemId>) -> LootContext<'static> {
    LootContext { tool: None, itself }
}

fn total(drops: &[(ItemId, u16)], item: ItemId) -> u32 {
    drops
        .iter()
        .filter(|(i, _)| *i == item)
        .map(|(_, c)| u32::from(*c))
        .sum()
}

#[test]
fn the_default_gives_the_block_itself() {
    let mut rng = Rng::new(1);
    let drops = LootTable::itself().roll(&bare(Some(ItemId(7))), &mut rng);
    assert_eq!(drops, vec![(ItemId(7), 1)]);
    assert!(LootTable::itself().roll(&bare(None), &mut rng).is_empty());
}

#[test]
fn weights_set_the_odds() {
    let loot = table(vec![
        entry(LootDrop::Item(ItemId(1)), 3, (1, 1)),
        entry(LootDrop::Nothing, 1, (0, 0)),
    ]);
    let mut rng = Rng::new(42);
    let mut drops = Vec::new();
    for _ in 0..4000 {
        drops.extend(loot.roll(&bare(None), &mut rng));
    }
    let given = total(&drops, ItemId(1));
    assert!((2800..3200).contains(&given), "{given}");
    let silent = table(vec![entry(LootDrop::Item(ItemId(1)), 0, (1, 1))]);
    assert!(silent.roll(&bare(None), &mut rng).is_empty());
}

#[test]
fn counts_stay_within_bounds_and_reach_both() {
    let loot = table(vec![entry(LootDrop::Item(ItemId(2)), 1, (2, 4))]);
    let mut rng = Rng::new(9);
    let counts: Vec<u16> = (0..500)
        .flat_map(|_| loot.roll(&bare(None), &mut rng))
        .map(|(_, count)| count)
        .collect();
    assert!(counts.iter().all(|c| (2..=4).contains(c)));
    assert!(counts.contains(&2) && counts.contains(&4));
    let zero = table(vec![entry(LootDrop::Item(ItemId(2)), 1, (0, 0))]);
    assert!(zero.roll(&bare(None), &mut rng).is_empty());
}

#[test]
fn tool_conditions_choose_the_entry() {
    let shears = ToolDef {
        kinds: vec![kind("shears")],
        ..ToolDef::default()
    };
    let mut with_shears = entry(LootDrop::Itself, 1, (1, 1));
    with_shears.conditions = vec![LootCondition::Tool(kind("shears"))];
    let mut without = entry(LootDrop::Item(ItemId(3)), 1, (1, 1));
    without.conditions = vec![LootCondition::Not(Box::new(LootCondition::Tool(kind(
        "shears",
    ))))];
    let loot = table(vec![with_shears, without]);
    let mut rng = Rng::new(5);
    let sheared = LootContext {
        tool: Some(&shears),
        itself: Some(ItemId(8)),
    };
    for _ in 0..50 {
        assert_eq!(loot.roll(&sheared, &mut rng), vec![(ItemId(8), 1)]);
        assert_eq!(
            loot.roll(&bare(Some(ItemId(8))), &mut rng),
            vec![(ItemId(3), 1)]
        );
    }
}

#[test]
fn pool_conditions_and_rolls_apply_to_the_whole_pool() {
    let mut loot = table(vec![entry(LootDrop::Item(ItemId(4)), 1, (1, 1))]);
    loot.pools[0].rolls = 3;
    let mut rng = Rng::new(3);
    assert_eq!(total(&loot.roll(&bare(None), &mut rng), ItemId(4)), 3);
    loot.pools[0].conditions = vec![LootCondition::Chance(0.0)];
    assert!(loot.roll(&bare(None), &mut rng).is_empty());
    loot.pools[0].conditions = vec![LootCondition::Chance(1.0)];
    assert_eq!(total(&loot.roll(&bare(None), &mut rng), ItemId(4)), 3);
}

#[test]
fn the_same_seed_gives_the_same_loot() {
    let loot = table(vec![
        entry(LootDrop::Item(ItemId(1)), 2, (1, 3)),
        entry(LootDrop::Item(ItemId(2)), 1, (1, 5)),
        entry(LootDrop::Nothing, 2, (0, 0)),
    ]);
    let run = |seed| {
        let mut rng = Rng::new(seed);
        (0..20)
            .flat_map(|_| loot.roll(&bare(None), &mut rng))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(77), run(77));
    assert_ne!(run(77), run(78));
}

#[test]
fn possible_items_list_each_item_once() {
    let loot = table(vec![
        entry(LootDrop::Item(ItemId(5)), 1, (1, 1)),
        entry(LootDrop::Itself, 1, (1, 1)),
        entry(LootDrop::Item(ItemId(5)), 1, (2, 2)),
        entry(LootDrop::Nothing, 1, (0, 0)),
    ]);
    assert_eq!(
        loot.possible_items(Some(ItemId(2))),
        vec![ItemId(2), ItemId(5)]
    );
    assert_eq!(loot.possible_items(None), vec![ItemId(5)]);
}

#[test]
fn only_required_tools_are_named() {
    let mut shears = entry(LootDrop::Itself, 1, (1, 1));
    shears.conditions = vec![LootCondition::Tool(kind("shears"))];
    let mut no_axe = entry(LootDrop::Nothing, 1, (0, 0));
    no_axe.conditions = vec![LootCondition::Not(Box::new(LootCondition::Tool(kind(
        "axe",
    ))))];
    let loot = table(vec![shears, no_axe]);
    assert_eq!(loot.tool_kinds(), vec![&kind("shears")]);
}
