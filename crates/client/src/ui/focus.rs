use super::model::Hit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// One step in this direction, in interface pixels (y grows downwards).
    pub fn offset(self) -> (i32, i32) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }
}

/// Next element in `direction` from `current` (gamepad / arrow navigation): the closest
/// one ahead, preferring those well aligned. Without a current element, the first one.
pub fn navigate(hits: &[Hit], current: Option<&Hit>, direction: Direction) -> Option<Hit> {
    let Some(current) = current else {
        return hits.first().cloned();
    };
    let (cx, cy) = current.rect.center();
    hits.iter()
        .filter(|hit| *hit != current)
        .filter_map(|hit| {
            let (x, y) = hit.rect.center();
            let (dx, dy) = (x - cx, y - cy);
            let (ahead, side) = match direction {
                Direction::Up => (-dy, dx),
                Direction::Down => (dy, dx),
                Direction::Left => (-dx, dy),
                Direction::Right => (dx, dy),
            };
            (ahead > 0).then_some((ahead + 3 * side.abs(), hit))
        })
        .min_by_key(|(score, _)| *score)
        .map(|(_, hit)| hit.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Rect;

    fn grid() -> Vec<Hit> {
        (0..6)
            .map(|i| Hit {
                id: "g".into(),
                index: i,
                rect: Rect::new((i % 3) as i32 * 20, (i / 3) as i32 * 20, 20, 20),
                kind: crate::ui::HitKind::Button,
            })
            .collect()
    }

    #[test]
    fn moves_to_the_neighbour_in_each_direction() {
        let hits = grid();
        let from = |i: usize, d| navigate(&hits, Some(&hits[i]), d).map(|h| h.index);
        assert_eq!(from(0, Direction::Right), Some(1));
        assert_eq!(from(0, Direction::Down), Some(3));
        assert_eq!(from(4, Direction::Left), Some(3));
        assert_eq!(from(4, Direction::Up), Some(1));
        assert_eq!(from(2, Direction::Right), None);
        assert_eq!(
            navigate(&hits, None, Direction::Down).map(|h| h.index),
            Some(0)
        );
    }
}
