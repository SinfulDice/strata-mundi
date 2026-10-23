use super::def::Anchor;

/// Rectangle in interface pixels (before scaling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w as i32 && py < self.y + self.h as i32
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w as i32 / 2, self.y + self.h as i32 / 2)
    }

    /// A child of size `size` attached to `anchor` of `self`, moved by `offset`.
    pub fn place(&self, anchor: Anchor, offset: (i32, i32), size: (u32, u32)) -> Rect {
        let (w, h) = (size.0 as i32, size.1 as i32);
        let (pw, ph) = (self.w as i32, self.h as i32);
        let (fx, fy) = match anchor {
            Anchor::TopLeft => (0, 0),
            Anchor::Top => (1, 0),
            Anchor::TopRight => (2, 0),
            Anchor::Left => (0, 1),
            Anchor::Center => (1, 1),
            Anchor::Right => (2, 1),
            Anchor::BottomLeft => (0, 2),
            Anchor::Bottom => (1, 2),
            Anchor::BottomRight => (2, 2),
        };
        let x = self.x + (pw - w) * fx / 2 + offset.0;
        let y = self.y + (ph - h) * fy / 2 + offset.1;
        Rect::new(x, y, size.0, size.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_place_children_on_any_screen_ratio() {
        let screen = Rect::new(0, 0, 400, 240);
        assert_eq!(
            screen.place(Anchor::Bottom, (0, -3), (182, 22)),
            Rect::new(109, 215, 182, 22)
        );
        assert_eq!(
            screen.place(Anchor::TopRight, (-4, 4), (10, 10)),
            Rect::new(386, 4, 10, 10)
        );
        assert_eq!(
            screen.place(Anchor::Center, (0, 0), (2, 2)),
            Rect::new(199, 119, 2, 2)
        );
        assert!(
            Rect::new(10, 10, 5, 5).contains(14, 14) && !Rect::new(10, 10, 5, 5).contains(15, 10)
        );
    }
}
