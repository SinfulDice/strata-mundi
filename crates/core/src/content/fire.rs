//! Fire and what it burns, as written in the pack.

use serde::Deserialize;

/// Oldest a fire can get: it then burns out (or keeps burning on its fuel).
pub const MAX_FIRE_AGE: u8 = 15;

/// `flammability` of a block, with Minecraft's scale: `catch` is how readily a fire
/// starts in the air next to it, `burn` how readily the block itself catches and burns
/// away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flammability {
    pub catch: u8,
    pub burn: u8,
}

/// `fire` of a block: makes it a fire that ages, spreads to flammable blocks and dies
/// out without fuel. Defaults are Minecraft's (normal difficulty).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FireDef {
    /// Least ticks between two steps.
    pub tick_rate: u32,
    /// Up to `tick_jitter − 1` ticks are added to each step, different for each place.
    pub tick_jitter: u32,
    /// Age, 1 to 15, at which a fire without fuel beneath may burn out.
    pub max_age: u8,
    /// Without any fuel next to it, a fire older than this dies.
    pub bare_age: u8,
    /// At its oldest, a fire dies with 1 chance in `burn_out_odds`.
    pub burn_out_odds: u32,
    /// A neighbour catches with `burn` chances in `side_odds` (sides) or `vertical_odds`
    /// (above and below).
    pub side_odds: u32,
    pub vertical_odds: u32,
    /// A neighbour that burns away becomes a fire with `rekindle_chance` chances in
    /// `age + rekindle_base`.
    pub rekindle_chance: u32,
    pub rekindle_base: u32,
    /// Air around a fire catches with `(catch + spread_bonus) / (age + spread_age_base)`
    /// chances in `spread_odds` (more odds for each block higher than one above).
    pub spread_bonus: u32,
    pub spread_age_base: u32,
    pub spread_odds: u32,
    /// The box of air a fire may light: `spread_reach` blocks to each side,
    /// `spread_below` below and `spread_above` above.
    pub spread_reach: u8,
    pub spread_below: u8,
    pub spread_above: u8,
}

impl Default for FireDef {
    fn default() -> Self {
        Self {
            tick_rate: 30,
            tick_jitter: 10,
            max_age: MAX_FIRE_AGE,
            bare_age: 3,
            burn_out_odds: 4,
            side_odds: 300,
            vertical_odds: 250,
            rekindle_chance: 5,
            rekindle_base: 10,
            spread_bonus: 54,
            spread_age_base: 30,
            spread_odds: 100,
            spread_reach: 1,
            spread_below: 1,
            spread_above: 4,
        }
    }
}

impl FireDef {
    /// Keeps every value usable and the box of air checked small: one fire step reads
    /// at most 7 × 7 × 12 blocks.
    pub fn clamped(self) -> Self {
        Self {
            tick_rate: self.tick_rate.clamp(1, 10_000),
            tick_jitter: self.tick_jitter.clamp(1, 1_000),
            max_age: self.max_age.clamp(1, MAX_FIRE_AGE),
            bare_age: self.bare_age.min(MAX_FIRE_AGE),
            burn_out_odds: self.burn_out_odds.clamp(1, 10_000),
            side_odds: self.side_odds.clamp(1, 100_000),
            vertical_odds: self.vertical_odds.clamp(1, 100_000),
            rekindle_chance: self.rekindle_chance.min(10_000),
            rekindle_base: self.rekindle_base.clamp(1, 10_000),
            spread_bonus: self.spread_bonus.min(10_000),
            spread_age_base: self.spread_age_base.clamp(1, 10_000),
            spread_odds: self.spread_odds.clamp(1, 100_000),
            spread_reach: self.spread_reach.min(3),
            spread_below: self.spread_below.min(3),
            spread_above: self.spread_above.min(8),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamping_keeps_odds_positive_and_the_box_small() {
        let wild = FireDef {
            tick_jitter: 0,
            max_age: 99,
            side_odds: 0,
            spread_reach: 50,
            spread_above: 50,
            ..FireDef::default()
        }
        .clamped();
        assert_eq!(wild.tick_jitter, 1);
        assert_eq!(wild.max_age, MAX_FIRE_AGE);
        assert_eq!(wild.side_odds, 1);
        assert_eq!((wild.spread_reach, wild.spread_above), (3, 8));
        assert_eq!(FireDef::default().clamped(), FireDef::default());
    }
}
