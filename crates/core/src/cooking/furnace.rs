//! One furnace: what it burns, what it cooks, and one tick of both.

use super::defs::{FurnaceDef, FurnaceTier};
use super::slots::{FURNACE_FUEL, FURNACE_INPUT, FURNACE_OUTPUT};
use super::{Cooking, CookingRecipe};
use crate::id::Identifier;
use crate::inventory::{Container, ItemRules, ItemStack, clear_empty};
use crate::upgrade::Effect;
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

/// How fast a furnace cooks, from its station, its tier and its upgrades.
#[derive(Debug, Clone, PartialEq)]
pub struct FurnaceStats {
    /// A cooking takes the recipe's time times this.
    pub time_factor: f64,
    /// A fuel burns its time times this.
    pub fuel_factor: f64,
    /// Results of one cooking, times the recipe's count.
    pub output: u16,
}

/// The stats furnace upgrades change: percent of a cooking's time, percent of the
/// cookings per fuel (both from 100), results per cooking (from 1).
const COOK_TIME: &str = "cook_time";
const FUEL_VALUE: &str = "fuel_value";
const OUTPUT: &str = "output";

impl FurnaceStats {
    /// `effects`: the bricks of its upgrades, each kind once (`slotted_effects`).
    pub fn new(furnace: &FurnaceDef, tier: &FurnaceTier, effects: &[Effect]) -> Self {
        let percent = |value: u16| f64::from(value.max(1)) / 100.0;
        let mut stats = [(COOK_TIME, 100.0), (FUEL_VALUE, 100.0), (OUTPUT, 1.0)];
        for effect in effects {
            let Effect::Stat { stat, op, value } = effect else {
                continue;
            };
            let named = stats
                .iter_mut()
                .find(|(name, _)| stat.namespace() == "strata" && stat.name() == *name);
            if let Some((_, current)) = named {
                *current = op.apply(*current, *value);
            }
        }
        let [(_, time), (_, value), (_, output)] = stats;
        let time =
            percent(tier.cook_time) / percent(furnace.speed) * f64::from(time.max(1.0)) / 100.0;
        let value = percent(tier.fuel_value) * f64::from(value.max(1.0)) / 100.0;
        Self {
            time_factor: time,
            fuel_factor: time * value,
            output: output.round().clamp(1.0, 64.0) as u16,
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

/// One furnace's rules: its speed, the kinds of recipes it cooks, and the recipes.
#[derive(Clone, Copy)]
pub struct Furnace<'a> {
    pub stats: &'a FurnaceStats,
    pub kinds: &'a [Identifier],
    pub cooking: &'a Cooking,
}

impl Furnace<'_> {
    /// The recipe of what lies in the input, and its result, when the whole result fits
    /// in the output.
    fn work(
        &self,
        slots: &Container,
        rules: &impl ItemRules,
    ) -> Option<(&CookingRecipe, ItemStack)> {
        let input = slots.get(FURNACE_INPUT)?;
        let recipe = self.cooking.recipe_for(input.item, self.kinds)?;
        let result = ItemStack::new(recipe.result, recipe.count * self.stats.output, rules);
        fits(slots.get(FURNACE_OUTPUT), &result, rules).then_some((recipe, result))
    }

    /// Ticks the cooking under way takes (0: nothing can be cooked now).
    pub fn cook_time(&self, slots: &Container, rules: &impl ItemRules) -> u32 {
        self.work(slots, rules)
            .map_or(0, |(recipe, _)| self.stats.cook_ticks(recipe))
    }

    /// Lights a new fuel when the fire is out and there is work.
    fn refuel(
        &self,
        state: &mut FurnaceState,
        slots: &mut Container,
        rules: &impl ItemRules,
    ) -> bool {
        if state.burn_left > 0 || self.work(slots, rules).is_none() {
            return false;
        }
        let Some(burn) = take_fuel(slots, self.cooking, rules) else {
            return false;
        };
        state.burn_left = self.stats.burn_ticks(burn);
        state.burn_total = state.burn_left;
        true
    }
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
/// it burns; a cooking stopped by an empty fuel cools down twice as fast as it ran. A
/// fuel burnt out with work left is replaced in the same tick, so the fire never blinks.
pub fn step(
    state: &mut FurnaceState,
    slots: &mut Container,
    furnace: &Furnace<'_>,
    rules: &impl ItemRules,
) -> Step {
    let mut changed = furnace.refuel(state, slots, rules);
    if state.burn_left > 0 {
        state.burn_left -= 1;
        match furnace.work(slots, rules) {
            Some((recipe, result)) => {
                state.progress += 1;
                if state.progress >= furnace.stats.cook_ticks(recipe) {
                    state.progress = 0;
                    let experience = recipe.experience;
                    cook(state, slots, result, experience);
                    changed = true;
                }
            }
            None => state.progress = 0,
        }
        changed |= furnace.refuel(state, slots, rules);
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

/// One input becomes the result; the experience counts once per cooking, whatever the
/// number of results (a double output gives no more experience).
fn cook(state: &mut FurnaceState, slots: &mut Container, result: ItemStack, experience: f32) {
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
    state.experience = (state.experience + experience).min(MAX_EXPERIENCE);
}
