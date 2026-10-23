use glam::DVec3;

const EPSILON: f64 = 1e-7;

/// Axis-aligned box in world coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: DVec3,
    pub max: DVec3,
}

impl Aabb {
    /// Box of `width` × `height` standing on `feet` (bottom centre).
    pub fn standing(feet: DVec3, width: f64, height: f64) -> Self {
        let half = width / 2.0;
        Self {
            min: DVec3::new(feet.x - half, feet.y, feet.z - half),
            max: DVec3::new(feet.x + half, feet.y + height, feet.z + half),
        }
    }

    /// Strict overlap: boxes that only touch do not overlap.
    pub fn overlaps(&self, other: &Aabb) -> bool {
        (0..3).all(|i| self.min[i] < other.max[i] && other.min[i] < self.max[i])
    }

    pub fn translated(self, offset: DVec3) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    /// Block coordinates the box overlaps on `axis` (touching faces excluded).
    fn cells(&self, axis: usize) -> std::ops::RangeInclusive<i32> {
        (self.min[axis] + EPSILON).floor() as i32..=(self.max[axis] - EPSILON).floor() as i32
    }

    /// How far the box can move by `distance` along `axis` before touching a solid block.
    pub fn sweep(&self, axis: usize, distance: f64, solid: &impl Fn(i32, i32, i32) -> bool) -> f64 {
        if distance == 0.0 {
            return 0.0;
        }
        let (a, b) = match axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        let blocked = |c: i32| {
            self.cells(a).any(|i| {
                self.cells(b).any(|j| {
                    let mut p = [0; 3];
                    p[axis] = c;
                    p[a] = i;
                    p[b] = j;
                    solid(p[0], p[1], p[2])
                })
            })
        };
        if distance > 0.0 {
            let first = (self.max[axis] - EPSILON).floor() as i32 + 1;
            let last = (self.max[axis] + distance - EPSILON).floor() as i32;
            (first..=last)
                .find(|&c| blocked(c))
                .map_or(distance, |c| f64::from(c) - self.max[axis])
        } else {
            let first = (self.min[axis] + EPSILON).floor() as i32 - 1;
            let last = (self.min[axis] + distance + EPSILON).floor() as i32;
            (last..=first)
                .rev()
                .find(|&c| blocked(c))
                .map_or(distance, |c| f64::from(c + 1) - self.min[axis])
        }
    }

    /// Whether the box overlaps any solid block.
    pub fn collides(&self, solid: &impl Fn(i32, i32, i32) -> bool) -> bool {
        self.cells(0)
            .any(|x| self.cells(1).any(|y| self.cells(2).any(|z| solid(x, y, z))))
    }

    /// Whether any block under the box (within `depth`) is solid.
    pub fn supported(&self, depth: f64, solid: &impl Fn(i32, i32, i32) -> bool) -> bool {
        self.sweep(1, -depth, solid) > -depth
    }
}
