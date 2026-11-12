//! Logs: an upright block and its two lying variants.

use super::block::{Axis, Block, Face, Orientation};
use super::registry::{BlockId, Registry};
use crate::id::Identifier;

/// Registers the upright block and its two lying variants `<name>__x` and `<name>__z`,
/// which have no item and share the upright block's name.
pub(super) fn insert_turned(
    blocks: &mut Registry<BlockId, Block>,
    id: &Identifier,
    upright: Block,
) -> bool {
    let variant_id = |axis: Axis| match axis {
        Axis::Y => Some(id.clone()),
        Axis::X => Identifier::new(id.namespace(), &format!("{}__x", id.name())).ok(),
        Axis::Z => Identifier::new(id.namespace(), &format!("{}__z", id.name())).ok(),
    };
    let Some(ids) = Axis::ALL
        .map(variant_id)
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .filter(|ids| ids.iter().all(|id| blocks.key(id).is_none()))
        .filter(|_| blocks.len() + 3 <= usize::from(u16::MAX) + 1)
    else {
        return false;
    };
    let first = blocks.len() as u16;
    let variants = [BlockId(first), BlockId(first + 1), BlockId(first + 2)];
    for (axis, id) in Axis::ALL.into_iter().zip(ids) {
        let block = Block {
            faces: Face::ALL.map(|face| upright.texture(axis.source_face(face))),
            has_item: upright.has_item && axis == Axis::Y,
            orientation: Some(Orientation { axis, variants }),
            ..upright.clone()
        };
        blocks.insert(id, block);
    }
    true
}
