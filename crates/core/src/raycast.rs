//! Which block the player is looking at: voxel traversal (Amanatides & Woo) that visits
//! every block crossed by the ray, in order, so it never skips a corner.

use crate::content::Face;
use crate::coords::BlockPos;
use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RayHit {
    pub block: BlockPos,
    /// Face of `block` the ray entered through: where a placed block goes.
    pub face: Face,
}

/// First block within `reach` for which `solid` is true.
pub fn raycast(
    origin: DVec3,
    direction: DVec3,
    reach: f64,
    mut solid: impl FnMut(BlockPos) -> bool,
) -> Option<RayHit> {
    if !origin.is_finite() || !reach.is_finite() {
        return None;
    }
    let direction = direction.try_normalize()?;
    let mut cell = origin.floor().as_ivec3();
    let step = direction.signum().as_ivec3();
    let next_boundary = |o: f64, c: i32, d: f64| {
        if d > 0.0 {
            f64::from(c + 1) - o
        } else {
            o - f64::from(c)
        }
    };
    let axes = [0, 1, 2];
    let mut t_max = axes.map(|i| {
        let d = direction[i];
        if d == 0.0 {
            f64::INFINITY
        } else {
            next_boundary(origin[i], cell[i], d) / d.abs()
        }
    });
    let t_delta = axes.map(|i| {
        if direction[i] == 0.0 {
            f64::INFINITY
        } else {
            1.0 / direction[i].abs()
        }
    });
    let mut face = None;
    loop {
        let pos = BlockPos::new(cell.x, cell.y, cell.z);
        if let Some(face) = face
            && solid(pos)
        {
            return Some(RayHit { block: pos, face });
        }
        let axis = (0..3)
            .min_by(|&a, &b| t_max[a].total_cmp(&t_max[b]))
            .unwrap_or(0);
        if t_max[axis] > reach {
            return None;
        }
        cell[axis] += step[axis];
        t_max[axis] += t_delta[axis];
        face = Some(entered_face(axis, step[axis]));
    }
}

/// Moving +X enters the next block through its west face, and so on.
fn entered_face(axis: usize, step: i32) -> Face {
    match (axis, step > 0) {
        (0, true) => Face::West,
        (0, false) => Face::East,
        (1, true) => Face::Down,
        (1, false) => Face::Up,
        (_, true) => Face::North,
        (_, false) => Face::South,
    }
}

impl Face {
    /// Unit offset toward the neighbour on this side.
    pub const fn offset(self) -> (i32, i32, i32) {
        match self {
            Self::Up => (0, 1, 0),
            Self::Down => (0, -1, 0),
            Self::North => (0, 0, -1),
            Self::South => (0, 0, 1),
            Self::East => (1, 0, 0),
            Self::West => (-1, 0, 0),
        }
    }
}

impl BlockPos {
    pub const fn neighbour(self, face: Face) -> Self {
        let (dx, dy, dz) = face.offset();
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(origin: DVec3, direction: DVec3, target: BlockPos) -> Option<RayHit> {
        raycast(origin, direction, 6.0, |pos| pos == target)
    }

    #[test]
    fn hits_the_block_in_front_through_the_facing_side() {
        let target = BlockPos::new(3, 0, 0);
        let result = hit(DVec3::new(0.5, 0.5, 0.5), DVec3::X, target);
        assert_eq!(
            result,
            Some(RayHit {
                block: target,
                face: Face::West
            })
        );
        assert_eq!(target.neighbour(Face::West), BlockPos::new(2, 0, 0));
    }

    #[test]
    fn crosses_column_borders_in_negative_coordinates() {
        let target = BlockPos::new(-17, 64, -1);
        let result = hit(DVec3::new(-14.5, 64.5, -0.5), DVec3::NEG_X, target);
        assert_eq!(
            result,
            Some(RayHit {
                block: target,
                face: Face::East
            })
        );
        let border = BlockPos::new(16, 70, 15);
        let result = hit(DVec3::new(15.5, 70.5, 15.5), DVec3::X, border);
        assert_eq!(result.map(|r| r.block), Some(border));
    }

    #[test]
    fn looking_down_hits_the_top_face() {
        let target = BlockPos::new(0, 10, 0);
        let result = hit(DVec3::new(0.5, 12.62, 0.5), DVec3::NEG_Y, target);
        assert_eq!(result.map(|r| r.face), Some(Face::Up));
    }

    #[test]
    fn nothing_beyond_reach_and_not_the_block_we_stand_in() {
        assert_eq!(
            hit(DVec3::new(0.5, 0.5, 0.5), DVec3::X, BlockPos::new(9, 0, 0)),
            None
        );
        assert_eq!(
            hit(DVec3::new(0.5, 0.5, 0.5), DVec3::X, BlockPos::new(0, 0, 0)),
            None
        );
    }

    #[test]
    fn diagonal_rays_do_not_slip_between_blocks() {
        let target = BlockPos::new(2, 2, 0);
        let result = hit(DVec3::new(0.5, 0.5, 0.5), DVec3::new(1.0, 1.0, 0.0), target);
        assert_eq!(result.map(|r| r.block), Some(target));
    }
}
