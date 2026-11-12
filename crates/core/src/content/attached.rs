//! Blocks hung on a neighbour (torches, lanterns): one variant per side they can lean on,
//! all sharing the item and name of the first one.

use super::block::{Block, Face, Placement};
use super::registry::{BlockId, Registry, TextureId};
use super::textures::TextureLibrary;
use crate::id::Identifier;
use crate::pack::PackError;

/// Sides an attached block may be placed against (see `Placement::Attached`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedSides {
    pub floor: bool,
    pub wall: bool,
    pub ceiling: bool,
}

impl AttachedSides {
    fn allows(&self, support: Face) -> bool {
        match support {
            Face::Down => self.floor,
            Face::Up => self.ceiling,
            _ => self.wall,
        }
    }
}

/// Member of a family of attached blocks: `support` is the side of the block holding it
/// (`Down` on the floor, `Up` under a ceiling, a wall otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attachment {
    pub support: Face,
    /// Variant leaning on each side, indexed by `Face::index`.
    pub variants: [Option<BlockId>; 6],
}

/// Order in which sides are tried when the aimed one cannot hold the block.
const FALLBACK: [Face; 6] = [
    Face::Down,
    Face::North,
    Face::South,
    Face::East,
    Face::West,
    Face::Up,
];

impl Attachment {
    pub fn on(&self, support: Face) -> Option<BlockId> {
        self.variants[support.index()]
    }

    /// The variant standing for the family (item, name): the first side in `FALLBACK`.
    pub fn family(&self) -> Option<BlockId> {
        FALLBACK.into_iter().find_map(|side| self.on(side))
    }

    /// Sides to try, the preferred one first.
    pub fn candidates(&self, preferred: Face) -> impl Iterator<Item = (Face, BlockId)> + '_ {
        std::iter::once(preferred)
            .chain(FALLBACK.into_iter().filter(move |&side| side != preferred))
            .filter_map(|side| Some((side, self.on(side)?)))
    }

    /// Wall variants are drawn leaning against `support`.
    pub fn wall(&self) -> Option<Face> {
        (!matches!(self.support, Face::Up | Face::Down)).then_some(self.support)
    }
}

fn suffix(support: Face) -> &'static str {
    match support {
        Face::Down => "floor",
        Face::Up => "ceiling",
        Face::North => "north",
        Face::South => "south",
        Face::East => "east",
        Face::West => "west",
    }
}

/// Registers one variant per allowed side: the family keeps `id`, the others are named
/// `<name>__<side>` and have no item. `false` when no side is allowed or a name is taken.
pub(super) fn insert_attached(
    blocks: &mut Registry<BlockId, Block>,
    id: &Identifier,
    block: Block,
    sides: AttachedSides,
    ceiling_texture: Option<TextureId>,
) -> bool {
    let sides: Vec<Face> = FALLBACK
        .into_iter()
        .filter(|&side| sides.allows(side))
        .collect();
    let named = sides.iter().enumerate().map(|(i, &side)| match i {
        0 => Some(id.clone()),
        _ => Identifier::new(id.namespace(), &format!("{}__{}", id.name(), suffix(side))).ok(),
    });
    let Some(ids) = named
        .collect::<Option<Vec<_>>>()
        .filter(|ids| !ids.is_empty() && ids.iter().all(|id| blocks.key(id).is_none()))
        .filter(|ids| blocks.len() + ids.len() <= usize::from(u16::MAX) + 1)
    else {
        return false;
    };
    let first = blocks.len() as u16;
    let mut variants = [None; 6];
    for (i, &side) in sides.iter().enumerate() {
        variants[side.index()] = Some(BlockId(first + i as u16));
    }
    for (i, (side, id)) in sides.into_iter().zip(ids).enumerate() {
        let faces = match (side, ceiling_texture) {
            (Face::Up, Some(layer)) => [layer; 6],
            _ => block.faces,
        };
        let variant = Block {
            faces,
            has_item: block.has_item && i == 0,
            attachment: Some(Attachment {
                support: side,
                variants,
            }),
            rooted: false,
            hanging: false,
            ..block.clone()
        };
        blocks.insert(id, variant);
    }
    true
}

/// Texture layer of the hanging variant, if the pack names one.
pub(super) fn ceiling_layer(
    placement: &Placement,
    textures: &TextureLibrary,
    source: &std::path::Path,
    errors: &mut Vec<PackError>,
) -> Option<TextureId> {
    let Placement::Attached {
        ceiling_texture: Some(texture),
        ..
    } = placement
    else {
        return None;
    };
    let layer = textures.layer(texture);
    if layer.is_none() {
        errors.push(PackError::invalid(
            source,
            format!("unknown texture `{texture}`"),
        ));
    }
    layer
}
