use crate::content::{
    Attachment, BlockId, Content, Face, FireDef, Flammability, Fluid, FluidLevel, FluidState,
    Orientation, Support,
};
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
    support: Option<Support>,
    orientation: Option<Orientation>,
    attachment: Option<Attachment>,
    /// A liquid may flow into it, sweeping it away (air, plants).
    open: bool,
    fluid: Option<FluidState>,
    flammability: Option<Flammability>,
    fire: Option<FireDef>,
    /// A liquid that puts fires out.
    douses: bool,
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
                support: block.support(),
                orientation: block.orientation,
                attachment: block.attachment,
                open: id == BlockId::AIR
                    || (!block.solid
                        && block.fluid.is_none()
                        && (block.replaceable || block.support().is_some())),
                fluid: block.fluid,
                flammability: block.flammability,
                fire: block.fire,
                douses: block.fluid.is_some_and(|state| {
                    content
                        .fluids
                        .get(&state.source)
                        .is_some_and(|fluid| fluid.puts_out_fire)
                }),
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

    /// Whether `block` can stand where `neighbour` describes its surroundings: rooted
    /// plants need a solid block beneath, hanging ones a solid or identical block above,
    /// torches their wall. `None` when the support is not loaded.
    pub fn is_supported(&self, block: BlockId, neighbour: impl Fn(Face) -> Cell) -> Option<bool> {
        let Some(support) = self.traits(block).support else {
            return Some(true);
        };
        let cell = neighbour(support.side);
        if support.chains && cell == Cell::Block(block) {
            return Some(true);
        }
        self.cell_is_solid(cell)
    }

    /// The variant set when `block` is placed against the `clicked` face of a block, if it
    /// can stand there: logs lie along the face's axis; torches lean on the clicked block,
    /// or else on the first other side able to hold them. `None` when nothing holds it or a
    /// support is not loaded.
    pub fn placed_variant(
        &self,
        block: BlockId,
        clicked: Face,
        neighbour: impl Fn(Face) -> Cell,
    ) -> Option<BlockId> {
        let traits = self.traits(block);
        if let Some(attachment) = traits.attachment {
            for (_, variant) in attachment.candidates(clicked.opposite()) {
                if self.is_supported(variant, &neighbour)? {
                    return Some(variant);
                }
            }
            return None;
        }
        let variant = traits
            .orientation
            .map_or(block, |o| o.along(clicked.axis()));
        (self.is_supported(variant, neighbour)?).then_some(variant)
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

    pub fn fire(&self, block: BlockId) -> Option<FireDef> {
        self.traits(block).fire
    }

    pub(super) fn flammability(&self, cell: Cell) -> Option<Flammability> {
        match cell {
            Cell::Block(block) => self.traits(block).flammability,
            _ => None,
        }
    }

    pub(super) fn douses(&self, cell: Cell) -> bool {
        matches!(cell, Cell::Block(block) if self.traits(block).douses)
    }

    pub(super) fn has_support_rule(&self, block: BlockId) -> bool {
        self.traits(block).support.is_some()
    }
}
