//! Tools and how long blocks take to break, the Minecraft way.

use super::Block;
use crate::id::Identifier;
use serde::Deserialize;

/// Simulation ticks per second: break times are counted in ticks.
pub const TICKS_PER_SECOND: u32 = 20;
/// Hardness × this many ticks breaks a block with a fitting tool (or one needing none)…
const HARVEST_FACTOR: f32 = 30.0;
/// … and × this many without the tool it requires.
const WRONG_TOOL_FACTOR: f32 = 100.0;
/// Speeds below this are read as this (a pack typo must not freeze a break forever).
const MIN_SPEED: f32 = 0.01;
const MAX_SPEED: f32 = 1000.0;

/// What an item mines and how well, its tier's values filled in (see `gear::ToolSpec`).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDef {
    /// Tool kinds it counts as (`strata:pickaxe`); several for a multi-tool.
    pub kinds: Vec<Identifier>,
    /// Mining level: 0 wood, 1 stone, 2 iron, 3 diamond; higher tiers come later.
    pub level: u8,
    /// Multiplies the breaking speed on the blocks of its kinds (a bare hand is 1).
    pub speed: f32,
    /// Instant blocks of its kinds this many blocks around the one broken break too.
    pub sweep: u8,
}

impl Default for ToolDef {
    fn default() -> Self {
        Self {
            kinds: Vec::new(),
            level: 0,
            speed: 1.0,
            sweep: 0,
        }
    }
}

/// `tool` of a block: the kind of tool that breaks it best.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "BlockTool", deny_unknown_fields)]
pub struct BlockTool {
    pub kind: Identifier,
    /// Lowest mining level that harvests it.
    #[serde(default)]
    pub level: u8,
    /// Without such a tool (of `level` at least), breaking is slow and drops nothing.
    #[serde(default)]
    pub required: bool,
}

impl ToolDef {
    pub fn is(&self, kind: &Identifier) -> bool {
        self.kinds.contains(kind)
    }

    /// The sweep of this tool around a broken `block`: only for blocks of its kinds.
    pub fn sweep_on(&self, block: &Block) -> u8 {
        let fits = block.tool.as_ref().is_some_and(|tool| self.is(&tool.kind));
        if fits { self.sweep } else { 0 }
    }

    /// Bounds a pack value: a speed of zero or less would never break anything.
    pub fn clamped(mut self) -> Self {
        self.speed = if self.speed.is_finite() {
            self.speed.clamp(MIN_SPEED, MAX_SPEED)
        } else {
            1.0
        };
        self
    }
}

impl Block {
    /// Whether breaking it with `held` (a bare hand when `None`) gives its loot.
    pub fn harvests(&self, held: Option<&ToolDef>) -> bool {
        match &self.tool {
            Some(tool) if tool.required => {
                held.is_some_and(|held| held.is(&tool.kind) && held.level >= tool.level)
            }
            _ => true,
        }
    }

    /// Ticks of steady hitting that break it with `held`: hardness × 30 / speed, or × 100
    /// without the tool it requires; 0 breaks at once, `None` never (bedrock).
    pub fn break_ticks(&self, held: Option<&ToolDef>) -> Option<u32> {
        let hardness = self.hardness;
        if !hardness.is_finite() || hardness < 0.0 {
            return None;
        }
        let speed = match (&self.tool, held) {
            (Some(tool), Some(held)) if held.is(&tool.kind) => held.speed.max(MIN_SPEED),
            _ => 1.0,
        };
        let factor = if self.harvests(held) {
            HARVEST_FACTOR
        } else {
            WRONG_TOOL_FACTOR
        };
        // Minecraft breaks at once when one tick does more than the whole block.
        let ticks = hardness * factor / speed;
        if ticks < 1.0 {
            return Some(0);
        }
        // The small margin keeps 15.000001 (a rounding error) at 15.
        Some((ticks - 1e-4).ceil() as u32)
    }
}

/// How far a break is, 0 (just started) to 255 (done): what the server tells, whatever
/// number of crack stages each client draws.
pub fn crack_progress(elapsed: f32, needed: f32) -> u8 {
    if needed <= 0.0 {
        return u8::MAX;
    }
    ((elapsed / needed).clamp(0.0, 1.0) * f32::from(u8::MAX)) as u8
}

/// Crack stage (0 to `stages - 1`) shown at `progress` (from `crack_progress`).
pub fn crack_stage(progress: u8, stages: usize) -> usize {
    (usize::from(progress) * stages / 256).min(stages.saturating_sub(1))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn block(hardness: f32, tool: Option<BlockTool>) -> Block {
        Block {
            hardness,
            tool,
            ..Block::air()
        }
    }

    fn kind(name: &str) -> Identifier {
        Identifier::new("strata", name).unwrap()
    }

    fn pickaxe_of(level: u8, speed: f32) -> ToolDef {
        ToolDef {
            kinds: vec![kind("pickaxe")],
            level,
            speed,
            sweep: 0,
        }
    }

    fn needs_pickaxe(level: u8) -> Option<BlockTool> {
        Some(BlockTool {
            kind: kind("pickaxe"),
            level,
            required: true,
        })
    }

    #[test]
    fn hardness_sets_the_time_by_hand() {
        assert_eq!(block(0.5, None).break_ticks(None), Some(15));
        assert_eq!(block(2.0, None).break_ticks(None), Some(60));
        assert_eq!(block(0.0, None).break_ticks(None), Some(0));
        assert_eq!(block(-1.0, None).break_ticks(None), None);
        assert_eq!(block(f32::NAN, None).break_ticks(None), None);
    }

    #[test]
    fn the_right_tool_is_faster_and_the_wrong_one_is_a_hand() {
        let stone = block(1.5, needs_pickaxe(0));
        assert_eq!(stone.break_ticks(None), Some(150));
        assert!(!stone.harvests(None));
        let wooden = pickaxe_of(0, 2.0);
        assert_eq!(stone.break_ticks(Some(&wooden)), Some(23));
        assert!(stone.harvests(Some(&wooden)));
        let shovel = ToolDef {
            kinds: vec![kind("shovel")],
            level: 3,
            speed: 8.0,
            sweep: 0,
        };
        assert_eq!(stone.break_ticks(Some(&shovel)), Some(150));
        assert!(!stone.harvests(Some(&shovel)));
    }

    #[test]
    fn a_low_level_tool_is_fast_but_harvests_nothing() {
        let diamond_ore = block(3.0, needs_pickaxe(2));
        let stone_pickaxe = pickaxe_of(1, 4.0);
        assert!(!diamond_ore.harvests(Some(&stone_pickaxe)));
        assert_eq!(diamond_ore.break_ticks(Some(&stone_pickaxe)), Some(75));
        let iron_pickaxe = pickaxe_of(2, 6.0);
        assert!(diamond_ore.harvests(Some(&iron_pickaxe)));
        assert_eq!(diamond_ore.break_ticks(Some(&iron_pickaxe)), Some(15));
    }

    #[test]
    fn an_optional_tool_only_speeds_up() {
        let log = block(
            2.0,
            Some(BlockTool {
                kind: kind("axe"),
                level: 0,
                required: false,
            }),
        );
        assert!(log.harvests(None));
        let axe = ToolDef {
            kinds: vec![kind("axe")],
            speed: 2.0,
            ..ToolDef::default()
        };
        assert_eq!(log.break_ticks(Some(&axe)), Some(30));
        let multi = ToolDef {
            kinds: vec![kind("pickaxe"), kind("axe")],
            ..axe
        };
        assert_eq!(log.break_ticks(Some(&multi)), Some(30));
    }

    #[test]
    fn a_very_fast_tool_breaks_at_once() {
        let leaves = block(0.2, None);
        assert_eq!(leaves.break_ticks(None), Some(6));
        let dirt = block(0.5, None);
        let fast = pickaxe_of(5, 100.0);
        assert_eq!(dirt.break_ticks(Some(&fast)), Some(15));
        let sand = block(0.5, needs_pickaxe(0));
        assert_eq!(sand.break_ticks(Some(&fast)), Some(0));
    }

    #[test]
    fn pack_speeds_are_bounded() {
        assert_eq!(pickaxe_of(0, 0.0).clamped().speed, MIN_SPEED);
        assert_eq!(pickaxe_of(0, f32::INFINITY).clamped().speed, 1.0);
        assert_eq!(pickaxe_of(0, 1e9).clamped().speed, MAX_SPEED);
    }

    #[test]
    fn crack_stages_cover_the_break() {
        assert_eq!(crack_progress(0.0, 20.0), 0);
        assert_eq!(crack_progress(10.0, 20.0), 127);
        assert_eq!(crack_progress(40.0, 20.0), 255);
        assert_eq!(crack_progress(5.0, 0.0), 255);
        let stage = |elapsed| crack_stage(crack_progress(elapsed, 20.0), 10);
        assert_eq!(stage(0.0), 0);
        assert_eq!(stage(1.9), 0);
        assert_eq!(stage(2.1), 1);
        assert_eq!(stage(19.9), 9);
        assert_eq!(stage(40.0), 9);
        assert_eq!(crack_stage(200, 0), 0);
        assert_eq!(crack_stage(255, 4), 3);
    }
}
