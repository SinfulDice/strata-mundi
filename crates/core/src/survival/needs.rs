//! What a player needs besides health: food and saturation, thirst, stamina and breath.
//! Effort adds exhaustion; exhaustion eats saturation, then food. Health comes back
//! when well fed, goes when starving or parched.

use super::health::Health;
use super::rules::SurvivalRules;
use crate::content::TICKS_PER_SECOND;
use serde::{Deserialize, Serialize};

/// World options: which needs count, and how fast they go down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurvivalOptions {
    #[serde(default = "yes")]
    pub hunger: bool,
    #[serde(default = "yes")]
    pub thirst: bool,
    #[serde(default = "yes")]
    pub stamina: bool,
    /// Percent of the normal speed at which hunger and thirst go down.
    #[serde(default = "hundred")]
    pub rate: u16,
}

fn yes() -> bool {
    true
}

fn hundred() -> u16 {
    100
}

impl Default for SurvivalOptions {
    fn default() -> Self {
        Self {
            hunger: true,
            thirst: true,
            stamina: true,
            rate: 100,
        }
    }
}

impl SurvivalOptions {
    /// The consumption speeds offered at world creation, in percent.
    pub const RATES: [u16; 5] = [25, 50, 100, 150, 200];

    fn factor(&self) -> f32 {
        f32::from(self.rate.min(1000)) / 100.0
    }
}

/// Effort of one tick, gathered while moving and acting.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Exertion {
    /// Blocks sprinted, and the ticks spent sprinting.
    pub sprinted: f32,
    pub sprint_ticks: u8,
    pub jumps: u8,
    pub sprint_jumps: u8,
    pub swum: f32,
    pub mined: u16,
    pub attacks: u8,
    pub dashes: u8,
}

/// What the body around the needs is like this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeedsContext {
    pub options: SurvivalOptions,
    /// The eyes are in a liquid.
    pub underwater: bool,
    /// Worn `strata:breath`, in seconds.
    pub extra_breath: f32,
    /// Share of the stamina heavy armour takes away (`Burden::stamina`).
    pub stamina_burden: f32,
}

/// Harm the needs deal (starving, parched, drowning).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeedHarm {
    Starvation,
    Dehydration,
    Drowning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    pub food: f32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub thirst: f32,
    pub thirst_exhaustion: f32,
    pub stamina: f32,
    /// Out of breath: no sprint until the stamina is back to `StaminaRules::resume`.
    #[serde(default)]
    pub winded: bool,
    /// Ticks of breath left under water.
    pub breath: u16,
    /// Ticks since stamina was last spent.
    #[serde(skip)]
    rest: u16,
    #[serde(skip)]
    timers: Timers,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Timers {
    regen: u16,
    starve: u16,
    dry: u16,
    drown: u16,
}

impl Needs {
    pub fn full(rules: &SurvivalRules) -> Self {
        Self {
            food: rules.hunger.max,
            saturation: rules.hunger.start_saturation.min(rules.hunger.max),
            exhaustion: 0.0,
            thirst: rules.thirst.max,
            thirst_exhaustion: 0.0,
            stamina: rules.stamina.max,
            winded: false,
            breath: breath_ticks(rules, 0.0),
            rest: 0,
            timers: Timers::default(),
        }
    }

    pub fn max_stamina(rules: &SurvivalRules, burden: f32) -> f32 {
        rules.stamina.max * (1.0 - burden.clamp(0.0, 0.9))
    }

    /// Adds the effort of a tick: exhaustion (hunger, thirst) and stamina spent.
    pub fn exert(&mut self, effort: &Exertion, rules: &SurvivalRules, context: &NeedsContext) {
        let hunger = &rules.hunger;
        let factor = context.options.factor();
        let exhaustion = effort.sprinted * hunger.sprint
            + f32::from(effort.jumps) * hunger.jump
            + f32::from(effort.sprint_jumps) * hunger.sprint_jump
            + effort.swum * hunger.swim
            + f32::from(effort.mined) * hunger.mine
            + f32::from(effort.attacks) * hunger.attack;
        self.add_exhaustion(exhaustion * factor, rules, &context.options);
        let stamina = &rules.stamina;
        let spent = f32::from(effort.sprint_ticks) * stamina.sprint
            + f32::from(effort.jumps + effort.sprint_jumps) * stamina.jump
            + f32::from(effort.attacks) * stamina.attack
            + f32::from(effort.dashes) * stamina.dash;
        if spent > 0.0 && context.options.stamina {
            self.stamina = (self.stamina - spent).max(0.0);
            self.rest = 0;
            if self.stamina <= 0.0 {
                self.winded = true;
            }
        }
    }

    fn add_exhaustion(&mut self, amount: f32, rules: &SurvivalRules, options: &SurvivalOptions) {
        if options.hunger {
            self.exhaustion += amount;
        }
        if options.thirst {
            self.thirst_exhaustion += amount * rules.thirst.share;
        }
    }

    /// One tick of the needs; heals `health` when fed, returns the harm they deal.
    pub fn tick(
        &mut self,
        health: &mut Health,
        rules: &SurvivalRules,
        context: &NeedsContext,
    ) -> Vec<(NeedHarm, f32)> {
        let options = context.options;
        if options.thirst {
            self.thirst_exhaustion += rules.thirst.idle * options.factor();
        }
        self.digest(rules);
        let mut harm = Vec::new();
        self.regenerate(health, rules, &options);
        let health_rules = &rules.health;
        let starving = options.hunger && self.food <= 0.0;
        if tick_timer(&mut self.timers.starve, starving, health_rules.starve_every)
            && health.current > health_rules.starve_floor
        {
            let room = health.current - health_rules.starve_floor;
            harm.push((NeedHarm::Starvation, health_rules.starve_damage.min(room)));
        }
        let parched = options.thirst && self.thirst <= 0.0;
        if tick_timer(&mut self.timers.dry, parched, rules.thirst.dry_every)
            && health.current > rules.thirst.floor
        {
            let room = health.current - rules.thirst.floor;
            harm.push((NeedHarm::Dehydration, rules.thirst.dry_damage.min(room)));
        }
        self.recover_stamina(rules, context);
        if self.breathe(rules, context) {
            harm.push((NeedHarm::Drowning, rules.breath.drown_damage));
        }
        harm
    }

    fn digest(&mut self, rules: &SurvivalRules) {
        let per_point = rules.hunger.per_point;
        while self.exhaustion >= per_point {
            self.exhaustion -= per_point;
            if self.saturation > 0.0 {
                self.saturation = (self.saturation - 1.0).max(0.0);
            } else {
                self.food = (self.food - 1.0).max(0.0);
            }
        }
        let per_point = rules.thirst.per_point;
        while self.thirst_exhaustion >= per_point {
            self.thirst_exhaustion -= per_point;
            self.thirst = (self.thirst - 1.0).max(0.0);
        }
    }

    /// Well fed (and not parched), health comes back, costing exhaustion; faster with
    /// full food and saturation left. Without hunger, it comes back at the normal pace.
    fn regenerate(
        &mut self,
        health: &mut Health,
        rules: &SurvivalRules,
        options: &SurvivalOptions,
    ) {
        let h = &rules.health;
        let wounded = health.current < health.max;
        let quenched = !options.thirst || self.thirst > 0.0;
        let saturated = options.hunger && self.food >= rules.hunger.max && self.saturation > 0.0;
        let fed = !options.hunger || self.food >= h.regen_food;
        let every = if saturated {
            h.saturated_every
        } else {
            h.regen_every
        };
        if tick_timer(&mut self.timers.regen, wounded && quenched && fed, every) {
            let healed = health.heal(h.regen_amount);
            // Healing tires (hunger) but does not dry: thirst has no saturation to soak it.
            if options.hunger {
                self.exhaustion += healed * h.regen_cost;
            }
        }
    }

    fn recover_stamina(&mut self, rules: &SurvivalRules, context: &NeedsContext) {
        let stamina = &rules.stamina;
        let max = Self::max_stamina(rules, context.stamina_burden);
        if !context.options.stamina {
            self.stamina = max;
            self.winded = false;
            return;
        }
        self.rest = self.rest.saturating_add(1);
        if self.rest > stamina.delay {
            self.stamina += stamina.regen;
        }
        self.stamina = self.stamina.min(max);
        if self.winded && self.stamina >= stamina.resume.min(max) {
            self.winded = false;
        }
    }

    /// Returns whether drowning hurts this tick.
    fn breathe(&mut self, rules: &SurvivalRules, context: &NeedsContext) -> bool {
        let max = breath_ticks(rules, context.extra_breath);
        if !context.underwater {
            self.breath = self.breath.saturating_add(rules.breath.refill).min(max);
            self.timers.drown = 0;
            return false;
        }
        self.breath = self.breath.min(max);
        if self.breath > 0 {
            self.breath -= 1;
            return false;
        }
        tick_timer(&mut self.timers.drown, true, rules.breath.drown_every)
    }

    /// Whether sprinting is allowed: not out of breath, not too hungry.
    pub fn can_sprint(&self, rules: &SurvivalRules, options: &SurvivalOptions) -> bool {
        let fed = !options.hunger || self.food > rules.hunger.sprint_min;
        let breath = !options.stamina || !self.winded;
        fed && breath
    }

    /// Eats or drinks: food and saturation (never above the food), thirst.
    pub fn nourish(&mut self, food: f32, saturation: f32, thirst: f32, rules: &SurvivalRules) {
        self.food = (self.food + food).clamp(0.0, rules.hunger.max);
        self.saturation = (self.saturation + saturation).clamp(0.0, self.food);
        self.thirst = (self.thirst + thirst).clamp(0.0, rules.thirst.max);
    }
}

/// Ticks of breath with `extra` seconds from equipment.
pub fn breath_ticks(rules: &SurvivalRules, extra: f32) -> u16 {
    let seconds = (rules.breath.seconds + extra).clamp(0.0, 3000.0);
    (seconds * TICKS_PER_SECOND as f32).round() as u16
}

/// Counts ticks while `on`; returns `true` (and starts again) every `every` ticks.
fn tick_timer(timer: &mut u16, on: bool, every: u16) -> bool {
    if !on {
        *timer = 0;
        return false;
    }
    *timer += 1;
    if *timer >= every {
        *timer = 0;
        return true;
    }
    false
}
