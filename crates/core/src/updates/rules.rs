use crate::content::{BlockId, Content, Fluid, FluidLevel, FluidState};
use std::collections::BTreeMap;

/// What block updates need to know about each block, indexed by `BlockId`.
#[derive(Debug, Clone, Default)]
pub struct UpdateRules {
    traits: Vec<Traits>,
    fluids: BTreeMap<BlockId, Fluid>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Traits {
    solid: bool,
    rooted: bool,
    hanging: bool,
    /// A liquid may flow into it, sweeping it away (air, plants).
    open: bool,
    fluid: Option<FluidState>,
}

/// A block as seen by the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// Its column is not loaded: whoever needs it waits.
    Unloaded,
    /// Beyond the world's border or height: behaves like a solid wall.
    Wall,
    Block(BlockId),
}

impl UpdateRules {
    pub fn new(content: &Content) -> Self {
        let traits = content
            .blocks
            .iter()
            .map(|(id, _, block)| Traits {
                solid: block.solid,
                rooted: block.rooted,
                hanging: block.hanging,
                open: id == BlockId::AIR
                    || (!block.solid
                        && block.fluid.is_none()
                        && (block.replaceable || block.rooted || block.hanging)),
                fluid: block.fluid,
            })
            .collect();
        Self {
            traits,
            fluids: content.fluids.clone(),
        }
    }

    fn traits(&self, block: BlockId) -> Traits {
        self.traits
            .get(usize::from(block.0))
            .copied()
            .unwrap_or_default()
    }

    pub fn is_solid(&self, block: BlockId) -> bool {
        self.traits(block).solid
    }

    pub fn fluid_state(&self, block: BlockId) -> Option<FluidState> {
        self.traits(block).fluid
    }

    pub fn fluid(&self, source: BlockId) -> Option<&Fluid> {
        self.fluids.get(&source)
    }

    /// Whether `block` can stand at `pos` given what is below and above it: rooted plants
    /// need a solid block beneath, hanging ones a solid or identical block above.
    /// `None` when a support is not loaded.
    pub fn is_supported(&self, block: BlockId, below: Cell, above: Cell) -> Option<bool> {
        let traits = self.traits(block);
        let ground = !traits.rooted || self.cell_is_solid(below)?;
        let ceiling =
            !traits.hanging || above == Cell::Block(block) || self.cell_is_solid(above)?;
        Some(ground && ceiling)
    }

    pub(super) fn cell_is_solid(&self, cell: Cell) -> Option<bool> {
        match cell {
            Cell::Unloaded => None,
            Cell::Wall => Some(true),
            Cell::Block(block) => Some(self.is_solid(block)),
        }
    }

    pub(super) fn is_open(&self, cell: Cell) -> bool {
        matches!(cell, Cell::Block(block) if self.traits(block).open)
    }

    /// The liquid in `cell`, if it is `source`'s.
    pub(super) fn level_of(&self, cell: Cell, source: BlockId) -> Option<FluidLevel> {
        match cell {
            Cell::Block(block) => self
                .fluid_state(block)
                .filter(|state| state.source == source)
                .map(|state| state.level),
            _ => None,
        }
    }

    pub(super) fn has_support_rule(&self, block: BlockId) -> bool {
        let traits = self.traits(block);
        traits.rooted || traits.hanging
    }
}
