//! `survival.ron`: the numbers of the survival gauges (health, hunger, thirst, stamina,
//! breath, falls, armour). The last pack that has the file wins; without one, the values
//! below (the base pack's) apply.

use crate::id::Identifier;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "Survival", deny_unknown_fields)]
pub struct SurvivalRules {
    pub health: HealthRules,
    pub hunger: HungerRules,
    pub thirst: ThirstRules,
    pub stamina: StaminaRules,
    pub breath: BreathRules,
    pub fall: FallRules,
    pub armor: ArmorRules,
    pub drinking: DrinkingRules,
    /// Ticks without damage after being hurt (the same kind of harm, or weaker, is ignored).
    pub invulnerability: u16,
    /// Ticks between two hits of suffocation while the eyes are inside a full block.
    pub suffocation_every: u16,
    pub suffocation_damage: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthRules {
    /// Health points without equipment (2 per heart).
    pub max: f32,
    /// Food from which health comes back on its own.
    pub regen_food: f32,
    pub regen_every: u16,
    /// Full food and saturation left: health comes back this often instead.
    pub saturated_every: u16,
    pub regen_amount: f32,
    /// Exhaustion each point healed costs.
    pub regen_cost: f32,
    /// Starving (food at 0): damage this often, down to `starve_floor`.
    pub starve_every: u16,
    pub starve_damage: f32,
    pub starve_floor: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HungerRules {
    pub max: f32,
    /// Saturation of a new character (and after a respawn).
    pub start_saturation: f32,
    /// Exhaustion that costs one point of saturation, or of food once saturation is gone.
    pub per_point: f32,
    /// Exhaustion per block sprinted, per jump, per sprinting jump, per block swum, per
    /// block broken, per attack.
    pub sprint: f32,
    pub jump: f32,
    pub sprint_jump: f32,
    pub swim: f32,
    pub mine: f32,
    pub attack: f32,
    /// At or below this food, the player cannot sprint.
    pub sprint_min: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThirstRules {
    pub max: f32,
    /// Thirst exhaustion that costs one point of thirst.
    pub per_point: f32,
    /// Share of the hunger exhaustion that also dries the player.
    pub share: f32,
    /// Thirst exhaustion added each tick, whatever the player does.
    pub idle: f32,
    /// Thirst at 0: damage this often, down to `floor`.
    pub dry_every: u16,
    pub dry_damage: f32,
    pub floor: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaminaRules {
    pub max: f32,
    /// Stamina back per tick, once `delay` ticks have passed since it was spent.
    pub regen: f32,
    pub delay: u16,
    /// Per tick of sprint, per jump, per attack, per dash.
    pub sprint: f32,
    pub jump: f32,
    pub attack: f32,
    pub dash: f32,
    /// Once out of breath, sprinting comes back at this much stamina.
    pub resume: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BreathRules {
    /// Seconds under water before drowning starts (worn `strata:breath` adds to it).
    pub seconds: f32,
    /// Ticks of breath back per tick at the air.
    pub refill: u16,
    pub drown_every: u16,
    pub drown_damage: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FallRules {
    /// Blocks fallen without harm.
    pub safe: f32,
    /// Damage per block beyond.
    pub per_block: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmorRules {
    /// Share of the damage each protection point takes away.
    pub per_point: f32,
    /// At most this share is taken away.
    pub max: f32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrinkingRules {
    /// The liquid one drinks from with an empty hand, sneaking.
    pub source: Identifier,
    /// Thirst one sip at a fresh source gives; at a salty one (`salty_biomes`), `salty_sip`.
    pub sip: f32,
    pub salty_sip: f32,
    /// Biomes whose water is salt water: bottles filled there must be purified.
    pub salty_biomes: Vec<Identifier>,
}

impl Default for SurvivalRules {
    fn default() -> Self {
        Self {
            health: HealthRules {
                max: 20.0,
                regen_food: 18.0,
                regen_every: 80,
                saturated_every: 10,
                regen_amount: 1.0,
                regen_cost: 6.0,
                starve_every: 80,
                starve_damage: 1.0,
                starve_floor: 1.0,
            },
            hunger: HungerRules {
                max: 20.0,
                start_saturation: 5.0,
                per_point: 4.0,
                sprint: 0.1,
                jump: 0.05,
                sprint_jump: 0.2,
                swim: 0.01,
                mine: 0.005,
                attack: 0.1,
                sprint_min: 6.0,
            },
            thirst: ThirstRules {
                max: 20.0,
                per_point: 4.0,
                share: 0.75,
                idle: 0.0025,
                dry_every: 80,
                dry_damage: 1.0,
                floor: 1.0,
            },
            stamina: StaminaRules {
                max: 100.0,
                regen: 1.0,
                delay: 15,
                sprint: 0.6,
                jump: 6.0,
                attack: 10.0,
                dash: 20.0,
                resume: 25.0,
            },
            breath: BreathRules {
                seconds: 15.0,
                refill: 5,
                drown_every: 20,
                drown_damage: 2.0,
            },
            fall: FallRules {
                safe: 3.0,
                per_block: 1.0,
            },
            armor: ArmorRules {
                per_point: 0.04,
                max: 0.8,
            },
            drinking: DrinkingRules {
                source: Identifier::builtin("strata", "water"),
                sip: 2.0,
                salty_sip: -1.0,
                salty_biomes: Vec::new(),
            },
            invulnerability: 10,
            suffocation_every: 10,
            suffocation_damage: 1.0,
        }
    }
}

impl SurvivalRules {
    /// Values a pack may not break the game with: no negative maximum, no zero period.
    pub fn clamped(mut self) -> Self {
        let positive = |value: &mut f32, least: f32| {
            *value = if value.is_finite() {
                value.max(least)
            } else {
                least
            };
        };
        positive(&mut self.health.max, 1.0);
        positive(&mut self.hunger.max, 1.0);
        positive(&mut self.thirst.max, 1.0);
        positive(&mut self.stamina.max, 1.0);
        positive(&mut self.hunger.per_point, 0.01);
        positive(&mut self.thirst.per_point, 0.01);
        positive(&mut self.breath.seconds, 0.0);
        positive(&mut self.fall.safe, 0.0);
        self.armor.max = self.armor.max.clamp(0.0, 1.0);
        for period in [
            &mut self.health.regen_every,
            &mut self.health.saturated_every,
            &mut self.health.starve_every,
            &mut self.thirst.dry_every,
            &mut self.breath.drown_every,
            &mut self.suffocation_every,
        ] {
            *period = (*period).max(1);
        }
        self
    }
}
