//! One furnace: what it burns, what it cooks, and one tick of both.

use super::defs::{FurnaceDef, FurnaceTier, ModuleDef};
use super::slots::{FURNACE_FUEL, FURNACE_INPUT, FURNACE_OUTPUT};
use super::{Cooking, CookingRecipe};
use crate::id::Identifier;
use crate::inventory::{Container, ItemRules, ItemStack, clear_empty};
use serde::{Deserialize, Serialize};

/// Experience a furnace keeps at most until it is taken (milestone 9).
const MAX_EXPERIENCE: f32 = 10_000.0;

/// What a furnace is doing, saved with its contents.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Furnace")]
pub struct FurnaceState {
    /// Ticks the current fuel still burns.
    pub burn_left: u32,
    /// Ticks the current fuel burnt in all (the flame shows `burn_left / burn_total`).
    pub burn_total: u32,
    /// Ticks the current cooking has run.
    pub progress: u32,
    /// Experience of the cookings done, kept for the player who takes the results.
    #[serde(default)]
    pub experience: f32,
}

impl FurnaceState {
    pub fn is_lit(&self) -> bool {
        self.burn_left > 0
    }
}

/// How fast a furnace cooks, from its station, its tier and its modules.
#[derive(Debug, Clone, PartialEq)]
pub struct FurnaceStats {
    pub kinds: Vec<Identifier>,
    /// A cooking takes the recipe's time times this.
    pub time_factor: f64,
    /// A fuel burns its time times this.
    pub fuel_factor: f64,
    /// Results of one cooking, times the recipe's count.
    pub output: u16,
}

impl FurnaceStats {
    /// Modules alike count once (the same effect never adds up).
    pub fn new(furnace: &FurnaceDef, tier: &FurnaceTier, modules: &[ModuleDef]) -> Self {
        let mut modules = modules.to_vec();
        modules.dedup();
        let percent = |value: u16| f64::from(value.max(1)) / 100.0;
        let mut time = percent(tier.cook_time) / percent(furnace.speed);
        let mut value = percent(tier.fuel_value);
        let mut output = 1;
        for module in modules {
            let ModuleDef::Furnace {
                cook_time,
                fuel_value,
                output: more,
            } = module;
            time *= percent(cook_time);
            value *= percent(fuel_value);
            output = output.max(u16::from(more.max(1)));
        }
        Self {
            kinds: furnace.kinds.clone(),
            time_factor: time,
            fuel_factor: time * value,
            output,
        }
    }

    /// Ticks one cooking of `recipe` takes here.
    pub fn cook_ticks(&self, recipe: &CookingRecipe) -> u32 {
        scaled(recipe.time, self.time_factor)
    }

    /// Ticks one fuel burns here.
    pub fn burn_ticks(&self, fuel: u32) -> u32 {
        scaled(fuel, self.fuel_factor)
    }
}

fn scaled(ticks: u32, factor: f64) -> u32 {
    ((f64::from(ticks) * factor).round() as u32).max(1)
}

/// What a tick changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Step {
    /// The slots changed (fuel burnt, a result made).
    pub slots: bool,
    /// Still burning or cooling: worth stepping again next tick.
    pub active: bool,
}

/// One tick: lights a fuel when there is something to cook and room for it, cooks while
/// it burns; a cooking stopped by an empty fuel cools down twice as fast as it ran.
pub fn step(
    state: &mut FurnaceState,
    slots: &mut Container,
    stats: &FurnaceStats,
    cooking: &Cooking,
    rules: &impl ItemRules,
) -> Step {
    let recipe = slots
        .get(FURNACE_INPUT)
        .and_then(|input| cooking.recipe_for(input.item, &stats.kinds));
    let result = recipe.map(|r| ItemStack::new(r.result, r.count * stats.output, rules));
    let room = result
        .as_ref()
        .is_some_and(|result| fits(slots.get(FURNACE_OUTPUT), result, rules));
    let mut changed = false;
    if state.burn_left == 0
        && room
        && let Some(burn) = take_fuel(slots, cooking, rules)
    {
        state.burn_left = stats.burn_ticks(burn);
        state.burn_total = state.burn_left;
        changed = true;
    }
    if state.burn_left > 0 {
        state.burn_left -= 1;
        match (room, recipe, result) {
            (true, Some(recipe), Some(result)) => {
                state.progress += 1;
                if state.progress >= stats.cook_ticks(recipe) {
                    state.progress = 0;
                    cook(state, slots, recipe, result, stats);
                    changed = true;
                }
            }
            _ => state.progress = 0,
        }
    } else {
        state.progress = state.progress.saturating_sub(2);
    }
    Step {
        slots: changed,
        active: state.burn_left > 0 || state.progress > 0,
    }
}

/// Whether the whole `result` fits on what the output slot holds.
fn fits(output: Option<&ItemStack>, result: &ItemStack, rules: &impl ItemRules) -> bool {
    match output {
        None => result.count <= rules.max_stack(result.item),
        Some(held) => {
            held.stacks_with(result) && held.count + result.count <= rules.max_stack(held.item)
        }
    }
}

/// Burns one fuel item; a full lava bucket leaves its empty bucket in the slot.
fn take_fuel(slots: &mut Container, cooking: &Cooking, rules: &impl ItemRules) -> Option<u32> {
    let slot = slots.slots.get_mut(FURNACE_FUEL)?;
    let stack = slot.as_mut()?;
    let burn = cooking.fuel(stack.item)?;
    let remainder = rules.remainder(stack.item);
    match remainder {
        Some(left) if stack.count == 1 => *slot = Some(ItemStack::new(left, 1, rules)),
        Some(_) => return None,
        None => {
            stack.count -= 1;
            clear_empty(slot);
        }
    }
    Some(burn)
}

fn cook(
    state: &mut FurnaceState,
    slots: &mut Container,
    recipe: &CookingRecipe,
    result: ItemStack,
    stats: &FurnaceStats,
) {
    if let Some(input) = slots.slots.get_mut(FURNACE_INPUT) {
        if let Some(stack) = input.as_mut() {
            stack.count -= 1;
        }
        clear_empty(input);
    }
    if let Some(output) = slots.slots.get_mut(FURNACE_OUTPUT) {
        match output.as_mut() {
            Some(held) => held.count += result.count,
            None => *output = Some(result),
        }
    }
    let gained = recipe.experience * f32::from(stats.output);
    state.experience = (state.experience + gained).min(MAX_EXPERIENCE);
}
