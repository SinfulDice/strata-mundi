//! Blocks with a front (furnaces): one variant per horizontal side the front may face,
//! and as many lit ones when the pack gives a lit look. All share the first one's item
//! and name.

use super::block::{Block, Face, Placement};
use super::registry::{BlockId, Registry, TextureId};
use super::textures::TextureLibrary;
use crate::id::Identifier;
use crate::pack::PackError;

/// Sides a front may face, in the order of `Facing::variants`.
pub const FRONTS: [Face; 4] = [Face::North, Face::South, Face::East, Face::West];

/// Member of a family of facing blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facing {
    pub front: Face,
    pub lit: bool,
    /// Unlit then lit variants, each indexed like `FRONTS`. Without a lit look, both rows
    /// are the unlit ones.
    pub variants: [[BlockId; 4]; 2],
}

impl Facing {
    /// The variant standing for the family (item, name): unlit, facing north.
    pub const fn family(&self) -> BlockId {
        self.variants[0][0]
    }

    /// The variant with its front toward `front` (a vertical face gives the north one),
    /// lit or not.
    pub fn variant(&self, front: Face, lit: bool) -> BlockId {
        self.variants[usize::from(lit)][front_index(front)]
    }

    /// The same front, lit or not.
    pub fn lit(&self, lit: bool) -> BlockId {
        self.variant(self.front, lit)
    }

    /// The front facing a player who looks along `yaw` (radians, 0 toward -Z).
    pub fn front_toward_viewer(yaw: f32) -> Face {
        let quarter = (yaw / std::f32::consts::FRAC_PI_2).round().rem_euclid(4.0) as u8;
        match quarter {
            0 => Face::South,
            1 => Face::East,
            2 => Face::North,
            _ => Face::West,
        }
    }
}

fn front_index(front: Face) -> usize {
    FRONTS.iter().position(|&f| f == front).unwrap_or(0)
}

/// Face of the north-facing block that ends up on `face` once its front is on `front`.
pub fn source_face(front: Face, face: Face) -> Face {
    let turns = match front {
        Face::East => 1,
        Face::South => 2,
        Face::West => 3,
        _ => 0,
    };
    let mut source = face;
    for _ in 0..turns {
        source = counter_clockwise(source);
    }
    source
}

/// Seen from above, a quarter turn against the clock: east goes north.
const fn counter_clockwise(face: Face) -> Face {
    match face {
        Face::East => Face::North,
        Face::North => Face::West,
        Face::West => Face::South,
        Face::South => Face::East,
        other => other,
    }
}

/// The lit look, resolved.
#[derive(Debug, Clone, Copy)]
pub(super) struct LitLook {
    pub front: TextureId,
    pub light_emission: u8,
}

/// Registers the north-facing block under `id` and the others as `<name>__south`,
/// `__east`, `__west`, then the lit ones as `<name>__lit`, `__lit_south`…; `false` when
/// a name is taken or the registry is full.
pub(super) fn insert_facing(
    blocks: &mut Registry<BlockId, Block>,
    id: &Identifier,
    block: Block,
    lit: Option<LitLook>,
) -> bool {
    let rows = if lit.is_some() { 2 } else { 1 };
    let Some(ids) = variant_ids(id, rows)
        .filter(|ids| ids.iter().all(|id| blocks.key(id).is_none()))
        .filter(|ids| blocks.len() + ids.len() <= usize::from(u16::MAX) + 1)
    else {
        return false;
    };
    let first = blocks.len() as u16;
    let row = |r: u16| std::array::from_fn(|i| BlockId(first + r * 4 + i as u16));
    let variants = [row(0), row(rows as u16 - 1)];
    for (index, id) in ids.into_iter().enumerate() {
        let (front, is_lit) = (FRONTS[index % 4], index >= 4);
        let mut faces = block.faces;
        let mut light_emission = block.light_emission;
        if let (true, Some(look)) = (is_lit, lit) {
            faces[Face::North.index()] = look.front;
            light_emission = look.light_emission;
        }
        let variant = Block {
            faces: Face::ALL.map(|face| faces[source_face(front, face).index()]),
            light_emission,
            has_item: block.has_item && index == 0,
            facing: Some(Facing {
                front,
                lit: is_lit,
                variants,
            }),
            ..block.clone()
        };
        blocks.insert(id, variant);
    }
    true
}

/// The lit look of a facing block, its texture resolved.
pub(super) fn lit_look(
    placement: &Placement,
    textures: &TextureLibrary,
    source: &std::path::Path,
    errors: &mut Vec<PackError>,
) -> Option<LitLook> {
    let Placement::Facing { lit: Some(lit) } = placement else {
        return None;
    };
    let Some(front) = textures.layer(&lit.front) else {
        errors.push(PackError::invalid(
            source,
            format!("unknown texture `{}`", lit.front),
        ));
        return None;
    };
    Some(LitLook {
        front,
        light_emission: lit.light_emission.min(15),
    })
}

fn variant_ids(id: &Identifier, rows: usize) -> Option<Vec<Identifier>> {
    let suffixes = ["", "__south", "__east", "__west"];
    let lit = ["__lit", "__lit_south", "__lit_east", "__lit_west"];
    suffixes
        .iter()
        .chain(lit.iter().take(4 * (rows - 1)))
        .map(|suffix| Identifier::new(id.namespace(), &format!("{}{suffix}", id.name())).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_front_is_drawn_on_the_side_it_faces() {
        assert_eq!(source_face(Face::North, Face::North), Face::North);
        assert_eq!(source_face(Face::East, Face::East), Face::North);
        assert_eq!(source_face(Face::South, Face::South), Face::North);
        assert_eq!(source_face(Face::West, Face::West), Face::North);
        assert_eq!(
            source_face(Face::South, Face::North),
            Face::South,
            "the back"
        );
        assert_eq!(source_face(Face::East, Face::Up), Face::Up);
    }

    #[test]
    fn the_front_faces_the_player_placing_it() {
        use std::f32::consts::{FRAC_PI_2, PI};
        assert_eq!(Facing::front_toward_viewer(0.0), Face::South);
        assert_eq!(Facing::front_toward_viewer(FRAC_PI_2), Face::East);
        assert_eq!(Facing::front_toward_viewer(PI), Face::North);
        assert_eq!(Facing::front_toward_viewer(-FRAC_PI_2), Face::West);
        assert_eq!(Facing::front_toward_viewer(2.0 * PI + 0.1), Face::South);
    }
}
