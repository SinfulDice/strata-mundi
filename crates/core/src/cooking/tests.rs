use super::*;
use crate::content::{Item, ItemKind, Registry, TextureId};
use crate::inventory::{Click, Container, ItemStack, PlayerInventory, SlotPolicy, SlotRef, Window};

const ORE: ItemId = ItemId(0);
const INGOT: ItemId = ItemId(1);
const COAL: ItemId = ItemId(2);
const LAVA: ItemId = ItemId(3);
const BUCKET: ItemId = ItemId(4);
const SPEED: ItemId = ItemId(5);
const DOUBLE: ItemId = ItemId(6);
const DIRT: ItemId = ItemId(7);
const LOG: ItemId = ItemId(8);
const CHARCOAL: ItemId = ItemId(9);
const STONE: ItemId = ItemId(10);
const BRICKS: ItemId = ItemId(11);

fn id(name: &str) -> Identifier {
    Identifier::new("test", name).unwrap()
}

fn items() -> Registry<ItemId, Item> {
    let names = [
        "ore", "ingot", "coal", "lava", "bucket", "speed", "double", "dirt", "log", "charcoal",
        "stone", "bricks",
    ];
    let mut items = Registry::default();
    for name in names {
        let module = match name {
            "speed" => Some(ModuleDef::Furnace {
                cook_time: 50,
                fuel_value: 100,
                output: 1,
            }),
            "double" => Some(ModuleDef::Furnace {
                cook_time: 100,
                fuel_value: 100,
                output: 2,
            }),
            _ => None,
        };
        let item = Item {
            max_stack: if name == "lava" || module.is_some() {
                1
            } else {
                64
            },
            category: None,
            kind: ItemKind::Flat(TextureId::MISSING),
            action: None,
            durability: None,
            equip: None,
            tool: None,
            remainder: (name == "lava").then_some(BUCKET),
            kit: None,
            module,
            gear: Default::default(),
        };
        items.insert(id(name), item);
    }
    items
}

fn recipe(name: &str, input: ItemId, result: ItemId, kind: &str) -> CookingRecipe {
    CookingRecipe {
        id: id(name),
        ingredient: Ingredient::item(input),
        result,
        count: 1,
        time: 200,
        experience: 0.5,
        kind: id(kind),
    }
}

fn cooking() -> Cooking {
    let recipes = vec![
        recipe("ingot", ORE, INGOT, "ore"),
        recipe("charcoal", LOG, CHARCOAL, "misc"),
    ];
    let cuttings = vec![CuttingRecipe {
        id: id("bricks"),
        ingredient: Ingredient::item(STONE),
        result: BRICKS,
        count: 1,
    }];
    let fuels = [(COAL, 1600), (LAVA, 20_000), (LOG, 300)].into();
    let tiers = [
        (id("stone"), FurnaceTier::default()),
        (
            id("iron"),
            FurnaceTier {
                cook_time: 80,
                fuel_value: 125,
                upgrade_slots: 1,
            },
        ),
    ]
    .into();
    Cooking::new(recipes, cuttings, (fuels, Some(1600)), tiers)
}

fn furnace(tier: &str, speed: u16) -> (FurnaceDef, FurnaceTier) {
    let def = FurnaceDef {
        family: id("furnace"),
        tier: id(tier),
        kinds: Vec::new(),
        speed,
    };
    let tier = *cooking().tier(&id(tier)).unwrap();
    (def, tier)
}

fn stats(tier: &str, modules: &[ModuleDef]) -> FurnaceStats {
    let (def, tier) = furnace(tier, 100);
    FurnaceStats::new(&def, &tier, modules)
}

fn slots(input: Option<(ItemId, u16)>, fuel: Option<(ItemId, u16)>) -> Container {
    let rules = items();
    let mut slots = Container::new(4);
    slots.slots[FURNACE_INPUT] = input.map(|(i, n)| ItemStack::new(i, n, &rules));
    slots.slots[FURNACE_FUEL] = fuel.map(|(i, n)| ItemStack::new(i, n, &rules));
    slots
}

fn count(slots: &Container, index: usize) -> u16 {
    slots.get(index).map_or(0, |s| s.count)
}

/// One tick of a furnace cooking every kind.
fn tick(state: &mut FurnaceState, slots: &mut Container, stats: &FurnaceStats) -> Step {
    let (cooking, rules) = (cooking(), items());
    let furnace = Furnace {
        stats,
        kinds: &[],
        cooking: &cooking,
    };
    step(state, slots, &furnace, &rules)
}

/// Steps until nothing burns or cools any more; returns the ticks run (the last idle one
/// included).
fn run(state: &mut FurnaceState, slots: &mut Container, stats: &FurnaceStats) -> u32 {
    let mut ticks = 1;
    while tick(state, slots, stats).active {
        ticks += 1;
        assert!(ticks < 1_000_000, "the furnace never stops");
    }
    ticks
}

#[test]
fn coal_cooks_eight_items_in_a_stone_furnace() {
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 20)), Some((COAL, 1)));
    let ticks = run(&mut state, &mut slots, &stats("stone", &[]));
    assert_eq!(count(&slots, FURNACE_OUTPUT), 8);
    assert_eq!(count(&slots, FURNACE_INPUT), 12);
    assert_eq!(count(&slots, FURNACE_FUEL), 0, "the coal burnt");
    assert_eq!(ticks, 1600);
    assert!(
        (state.experience - 4.0).abs() < 1e-4,
        "experience is stored"
    );
}

#[test]
fn a_better_tier_cooks_faster_and_burns_less() {
    let stone = stats("stone", &[]);
    let iron = stats("iron", &[]);
    let ore = recipe("ingot", ORE, INGOT, "ore");
    assert_eq!(stone.cook_ticks(&ore), 200);
    assert_eq!(iron.cook_ticks(&ore), 160);
    let per_coal = |s: &FurnaceStats| s.burn_ticks(1600) / s.cook_ticks(&ore);
    assert_eq!(per_coal(&stone), 8);
    assert_eq!(per_coal(&iron), 10);
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 20)), Some((COAL, 1)));
    run(&mut state, &mut slots, &iron);
    assert_eq!(count(&slots, FURNACE_OUTPUT), 10);
}

#[test]
fn a_faster_station_burns_its_fuel_as_much_faster() {
    let (def, tier) = furnace("stone", 200);
    let blast = FurnaceStats::new(&def, &tier, &[]);
    let ore = recipe("ingot", ORE, INGOT, "ore");
    assert_eq!(blast.cook_ticks(&ore), 100);
    assert_eq!(blast.burn_ticks(1600) / blast.cook_ticks(&ore), 8);
}

#[test]
fn nothing_burns_without_something_to_cook() {
    let mut state = FurnaceState::default();
    let mut idle = slots(Some((DIRT, 5)), Some((COAL, 3)));
    let done = tick(&mut state, &mut idle, &stats("stone", &[]));
    assert!(!done.active && !done.slots);
    assert_eq!(
        count(&idle, FURNACE_FUEL),
        3,
        "dirt does not cook: the coal stays"
    );
    let mut empty = slots(None, Some((COAL, 3)));
    tick(&mut state, &mut empty, &stats("stone", &[]));
    assert_eq!(count(&empty, FURNACE_FUEL), 3);
}

#[test]
fn a_full_output_stops_the_cooking() {
    let rules = items();
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 20)), Some((COAL, 5)));
    slots.slots[FURNACE_OUTPUT] = Some(ItemStack::new(INGOT, 63, &rules));
    run(&mut state, &mut slots, &stats("stone", &[]));
    assert_eq!(count(&slots, FURNACE_OUTPUT), 64);
    assert_eq!(count(&slots, FURNACE_INPUT), 19, "one cooked, then blocked");
    assert_eq!(count(&slots, FURNACE_FUEL), 4, "no more coal lit once full");
    let mut other = slots.clone();
    other.slots[FURNACE_OUTPUT] = Some(ItemStack::new(CHARCOAL, 1, &rules));
    let mut fresh = FurnaceState::default();
    let ticks = run(&mut fresh, &mut other, &stats("stone", &[]));
    assert_eq!(ticks, 1, "another item in the output blocks it at once");
}

#[test]
fn a_cooking_cut_short_cools_down() {
    let stats = stats("stone", &[]);
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 2)), Some((LOG, 1)));
    run(&mut state, &mut slots, &stats);
    assert_eq!(
        count(&slots, FURNACE_OUTPUT),
        1,
        "a log burns 300 ticks: one and a half"
    );
    assert_eq!(count(&slots, FURNACE_INPUT), 1);
    assert_eq!(state.progress, 0, "the half done cooled down");
    let mut warm = FurnaceState {
        progress: 10,
        ..FurnaceState::default()
    };
    let mut cold = Container::new(4);
    tick(&mut warm, &mut cold, &stats);
    assert_eq!(warm.progress, 8);
}

#[test]
fn a_lava_bucket_leaves_its_bucket() {
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 1)), Some((LAVA, 1)));
    tick(&mut state, &mut slots, &stats("stone", &[]));
    assert_eq!(slots.get(FURNACE_FUEL).map(|s| s.item), Some(BUCKET));
    assert!(state.is_lit());
}

#[test]
fn modules_speed_up_and_double_the_output_once_each() {
    let rules = items();
    let module = |item: ItemId| rules.get(item).unwrap().module.unwrap();
    let speed = stats("stone", &[module(SPEED)]);
    let ore = recipe("ingot", ORE, INGOT, "ore");
    assert_eq!(speed.cook_ticks(&ore), 100);
    assert_eq!(
        stats("stone", &[module(SPEED), module(SPEED)]).cook_ticks(&ore),
        100,
        "the same module does not add up"
    );
    let double = stats("stone", &[module(DOUBLE)]);
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 2)), Some((COAL, 1)));
    run(&mut state, &mut slots, &double);
    assert_eq!(
        count(&slots, FURNACE_OUTPUT),
        4,
        "two ores give four ingots"
    );
    let mut nearly = self::slots(Some((ORE, 2)), Some((COAL, 1)));
    nearly.slots[FURNACE_OUTPUT] = Some(ItemStack::new(INGOT, 63, &rules));
    run(&mut FurnaceState::default(), &mut nearly, &double);
    assert_eq!(
        count(&nearly, FURNACE_OUTPUT),
        63,
        "a double output needs room for two"
    );
}

#[test]
fn shift_clicks_go_to_the_right_furnace_slot() {
    let (cooking, rules) = (cooking(), items());
    let policy = FurnaceSlots {
        cooking: &cooking,
        items: &rules,
        kinds: &[],
    };
    let mut player = PlayerInventory::default();
    for (slot, item, n) in [(0, ORE, 10), (1, COAL, 5), (2, SPEED, 1), (3, DIRT, 7)] {
        player.slots.slots[slot] = Some(ItemStack::new(item, n, &rules));
    }
    let mut furnace = Container::new(4);
    let mut window = Window {
        player: &mut player,
        open: Some(&mut furnace),
        craft: None,
        policy: Some(&policy),
    };
    for slot in 0..4 {
        window.click(SlotRef::player(slot), Click::Transfer, &rules);
    }
    assert_eq!(count(&furnace, FURNACE_INPUT), 10);
    assert_eq!(count(&furnace, FURNACE_FUEL), 5);
    assert_eq!(furnace.get(FURNACE_MODULES).map(|s| s.item), Some(SPEED));
    assert_eq!(count(&furnace, FURNACE_OUTPUT), 0);
    assert_eq!(count(&player.slots, 3), 7, "dirt has no slot here");
}

#[test]
fn furnace_slots_refuse_what_does_not_belong() {
    let (cooking, rules) = (cooking(), items());
    let policy = FurnaceSlots {
        cooking: &cooking,
        items: &rules,
        kinds: &[],
    };
    let slots = Container::new(5);
    let stack = |item| ItemStack::new(item, 1, &rules);
    assert!(policy.accepts(&slots, FURNACE_INPUT, &stack(DIRT)));
    assert!(!policy.accepts(&slots, FURNACE_FUEL, &stack(DIRT)));
    assert!(!policy.accepts(&slots, FURNACE_OUTPUT, &stack(ORE)));
    assert!(!policy.accepts(&slots, FURNACE_MODULES, &stack(COAL)));
    let mut with_speed = slots.clone();
    with_speed.slots[FURNACE_MODULES] = Some(stack(SPEED));
    assert!(
        !policy.accepts(&with_speed, FURNACE_MODULES + 1, &stack(SPEED)),
        "the same module twice"
    );
    assert!(policy.accepts(&with_speed, FURNACE_MODULES + 1, &stack(DOUBLE)));
    let mut log_in = slots.clone();
    log_in.slots[FURNACE_INPUT] = Some(stack(ORE));
    assert_eq!(policy.route(&log_in, &stack(LOG)), vec![FURNACE_FUEL]);
    assert_eq!(policy.route(&slots, &stack(LOG)), vec![FURNACE_INPUT]);
}

#[test]
fn kinds_choose_the_recipes_of_a_station() {
    let cooking = cooking();
    assert!(cooking.recipe_for(ORE, &[id("ore")]).is_some());
    assert!(cooking.recipe_for(LOG, &[id("ore")]).is_none());
    assert!(
        cooking.recipe_for(LOG, &[]).is_some(),
        "no kind: everything"
    );
    assert_eq!(cooking.cuts_of(STONE).len(), 1);
    assert!(cooking.cuts_of(ORE).is_empty());
}

#[test]
fn the_stonecutter_cuts_the_chosen_variant() {
    use crate::inventory::CraftGrid;
    let (cooking, rules) = (cooking(), items());
    let mut grid = CraftGrid::new(1);
    grid.slots.slots[0] = Some(ItemStack::new(STONE, 3, &rules));
    let choices = cut_choices(&cooking, &grid);
    assert_eq!(choices.len(), 1);
    let choice = Some(choices[0].0);
    assert!(
        cut_result(&cooking, &grid, None, &rules).is_none(),
        "nothing chosen yet"
    );
    let mut player = PlayerInventory::default();
    let mut window = Window {
        player: &mut player,
        open: None,
        craft: Some(&mut grid),
        policy: None,
    };
    assert_eq!(
        take_cut(&mut window, &cooking, choice, Click::Primary, &rules),
        1
    );
    assert_eq!(
        window.player.cursor.as_ref().map(|s| (s.item, s.count)),
        Some((BRICKS, 1))
    );
    assert_eq!(
        take_cut(&mut window, &cooking, choice, Click::Transfer, &rules),
        2
    );
    assert_eq!(window.player.slots.count(BRICKS), 2);
    assert!(
        window.craft.as_ref().unwrap().slots.get(0).is_none(),
        "the stone is used up"
    );
}

#[test]
fn the_fire_never_blinks_between_two_fuels() {
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 10)), Some((COAL, 2)));
    let stats = stats("stone", &[]);
    let mut lit = Vec::new();
    while tick(&mut state, &mut slots, &stats).active {
        lit.push(state.is_lit());
    }
    let changes = lit.windows(2).filter(|pair| pair[0] != pair[1]).count();
    assert_eq!(changes, 0, "lit from the first tick to the last");
    assert_eq!(count(&slots, FURNACE_OUTPUT), 10);
    assert_eq!(count(&slots, FURNACE_FUEL), 0, "the second coal was lit");
}

#[test]
fn a_double_output_gives_no_more_experience() {
    let rules = items();
    let double = rules.get(DOUBLE).unwrap().module.unwrap();
    let mut state = FurnaceState::default();
    let mut slots = slots(Some((ORE, 2)), Some((COAL, 1)));
    run(&mut state, &mut slots, &stats("stone", &[double]));
    assert!((state.experience - 1.0).abs() < 1e-4, "0.5 per cooking");
}
