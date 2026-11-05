//! Fire, close to Minecraft's: it ages at each step, burns its flammable neighbours away,
//! lights the air next to flammable blocks, and dies out without fuel. Chances come from
//! a hash of the place and tick, so a world burns the same way every time.

use super::rules::Cell;
use super::{BlockAccess, BlockChange, BlockUpdates, WAIT_TICKS, cell};
use crate::content::{BlockId, Face, FireDef, MAX_FIRE_AGE};
use crate::coords::BlockPos;
use serde::{Deserialize, Serialize};

/// Extra ticks added to a fire's `tick_rate`, different for each place and step.
const TICK_JITTER: u64 = 10;
/// Odds against a fire's neighbour catching: sides, then above or below.
const SIDE_ODDS: u32 = 300;
const VERTICAL_ODDS: u32 = 250;
/// Added to the `catch` of the air around a fire (Minecraft's 40 plus its normal
/// difficulty).
const SPREAD_BONUS: u32 = 54;

/// World option: fire may spread and destroy what burns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FireSettings {
    /// Fire lights new fires around it.
    pub spread: bool,
    /// Burning blocks are consumed.
    pub destroy: bool,
}

impl Default for FireSettings {
    fn default() -> Self {
        Self {
            spread: true,
            destroy: true,
        }
    }
}

/// Deterministic chances for one step of one fire.
struct Roll(u64);

impl Roll {
    fn new(pos: BlockPos, tick: u64) -> Self {
        let place = (i64::from(pos.x) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (i64::from(pos.y) as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ (i64::from(pos.z) as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
        Self(place ^ tick.wrapping_mul(0xFF51_AFD7_ED55_8CCD))
    }

    /// splitmix64.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0 to `n − 1`.
    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n.max(1))) as u32
    }
}

impl BlockUpdates {
    pub(super) fn plan_fire(&mut self, pos: BlockPos, fire: FireDef) {
        let jitter = Roll::new(pos, self.now).next() % TICK_JITTER;
        self.plan(pos, self.now + u64::from(fire.tick_rate) + jitter);
    }

    /// Whether a fire may burn at `pos`: on solid ground or next to something flammable,
    /// away from water and rain. `None` when a neighbour is not loaded.
    pub fn fire_can_stay(&self, world: &impl BlockAccess, pos: BlockPos) -> Option<bool> {
        let mut ground = false;
        let mut fuel = false;
        for face in Face::ALL {
            let neighbour = cell(world, pos.neighbour(face));
            if neighbour == Cell::Unloaded {
                return None;
            }
            if self.rules.douses(neighbour) {
                return Some(false);
            }
            fuel |= self.rules.flammability(neighbour).is_some();
            if face == Face::Down {
                ground = self.rules.cell_is_solid(neighbour)?;
            }
        }
        Some((ground || fuel) && !world.rains_on(pos))
    }

    pub(super) fn update_fire(
        &mut self,
        world: &mut impl BlockAccess,
        pos: BlockPos,
        changes: &mut Vec<BlockChange>,
    ) {
        let Cell::Block(block) = cell(world, pos) else {
            return;
        };
        let Some(fire) = self.rules.fire(block) else {
            return;
        };
        match self.fire_can_stay(world, pos) {
            None => {
                self.plan(pos, self.now + WAIT_TICKS);
                return;
            }
            Some(false) => {
                self.set_block(world, pos, BlockId::AIR, changes);
                return;
            }
            Some(true) => {}
        }
        let mut roll = Roll::new(pos, self.now);
        let old = self.fire_ages.get(&pos).copied().unwrap_or(0);
        let age = (old + (roll.below(3) / 2) as u8).min(fire.max_age);
        if self.burns_out(world, pos, age, fire, &mut roll) {
            self.set_block(world, pos, BlockId::AIR, changes);
            return;
        }
        self.fire_ages.insert(pos, age);
        self.plan_fire(pos, fire);
        if self.fire.destroy {
            for face in Face::ALL {
                let odds = match face {
                    Face::Up | Face::Down => VERTICAL_ODDS,
                    _ => SIDE_ODDS,
                };
                let target = pos.neighbour(face);
                self.burn(world, (target, block), odds, age, &mut roll, changes);
            }
        }
        if self.fire.spread && age < fire.max_age {
            self.spread(world, (pos, block), age, &mut roll, changes);
        }
    }

    /// Without fuel nearby a fire soon dies; at its oldest it may die unless it stands on
    /// fuel that can still burn away.
    fn burns_out(
        &self,
        world: &impl BlockAccess,
        pos: BlockPos,
        age: u8,
        fire: FireDef,
        roll: &mut Roll,
    ) -> bool {
        let on_fuel = self
            .rules
            .flammability(cell(world, pos.neighbour(Face::Down)))
            .is_some();
        let fuel_near = Face::ALL.into_iter().any(|face| {
            let neighbour = cell(world, pos.neighbour(face));
            self.rules.flammability(neighbour).is_some()
        });
        if !fuel_near && age > 3 {
            return true;
        }
        let burning_forever = on_fuel && self.fire.destroy;
        age >= fire.max_age && !burning_forever && roll.below(4) == 0
    }

    /// A flammable neighbour may catch: it burns away, sometimes into a new fire.
    fn burn(
        &mut self,
        world: &mut impl BlockAccess,
        (target, fire): (BlockPos, BlockId),
        odds: u32,
        age: u8,
        roll: &mut Roll,
        changes: &mut Vec<BlockChange>,
    ) {
        let Some(flammability) = self.rules.flammability(cell(world, target)) else {
            return;
        };
        if roll.below(odds) >= u32::from(flammability.burn) {
            return;
        }
        let rekindles =
            self.fire.spread && roll.below(u32::from(age) + 10) < 5 && !world.rains_on(target);
        if rekindles {
            self.light(world, target, fire, age, roll, changes);
        } else {
            self.set_block(world, target, BlockId::AIR, changes);
        }
    }

    /// The air around (up to four blocks above) may catch from the flammable blocks it
    /// touches, less often the higher and the older the fire. Fires at their oldest no
    /// longer do: ages only grow along a chain of fires, so every fire ends.
    fn spread(
        &mut self,
        world: &mut impl BlockAccess,
        (pos, fire): (BlockPos, BlockId),
        age: u8,
        roll: &mut Roll,
        changes: &mut Vec<BlockChange>,
    ) {
        for dx in -1..=1 {
            for dz in -1..=1 {
                for dy in -1..=4 {
                    if (dx, dy, dz) == (0, 0, 0) {
                        continue;
                    }
                    let target = BlockPos::new(pos.x + dx, pos.y + dy, pos.z + dz);
                    let odds = 100 + 100 * (dy - 1).max(0).unsigned_abs();
                    let chance = self.catch_chance(world, target, age);
                    if chance > 0
                        && roll.below(odds) <= chance
                        && self.fire_can_stay(world, target) == Some(true)
                    {
                        self.light(world, target, fire, age, roll, changes);
                    }
                }
            }
        }
    }

    /// How readily the air at `pos` catches fire: 0 if it is not air or touches nothing
    /// flammable.
    fn catch_chance(&self, world: &impl BlockAccess, pos: BlockPos, age: u8) -> u32 {
        if cell(world, pos) != Cell::Block(BlockId::AIR) {
            return 0;
        }
        let catch = Face::ALL
            .into_iter()
            .filter_map(|face| self.rules.flammability(cell(world, pos.neighbour(face))))
            .map(|flammability| u32::from(flammability.catch))
            .max()
            .unwrap_or(0);
        if catch == 0 {
            return 0;
        }
        (catch + SPREAD_BONUS) / (u32::from(age) + 30)
    }

    /// A new fire, a little older than the one it comes from.
    fn light(
        &mut self,
        world: &mut impl BlockAccess,
        pos: BlockPos,
        fire: BlockId,
        age: u8,
        roll: &mut Roll,
        changes: &mut Vec<BlockChange>,
    ) {
        if self.set_block(world, pos, fire, changes) && world.block(pos) == Some(fire) {
            let age = (age + (roll.below(5) / 4) as u8).min(MAX_FIRE_AGE);
            self.fire_ages.insert(pos, age);
        }
    }
}
