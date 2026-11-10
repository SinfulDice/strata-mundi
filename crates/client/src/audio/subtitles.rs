//! Subtitles: one line per sound heard lately, with where it came from; a sound heard
//! again refreshes its line instead of adding one.

use super::spatial::Direction;

/// Seconds a line stays after the sound was last heard.
pub const LIFETIME: f32 = 3.0;
/// Lines shown at most (the oldest go first).
pub const MAX_LINES: usize = 6;

#[derive(Debug, Clone, PartialEq)]
pub struct Subtitle {
    /// Translation key of the sound's text.
    pub key: String,
    pub direction: Direction,
    /// Seconds since last heard.
    pub age: f32,
}

#[derive(Debug, Default)]
pub struct Subtitles {
    lines: Vec<Subtitle>,
}

impl Subtitles {
    pub fn heard(&mut self, key: &str, direction: Direction) {
        if let Some(line) = self.lines.iter_mut().find(|line| line.key == key) {
            line.direction = direction;
            line.age = 0.0;
        } else {
            if self.lines.len() == MAX_LINES {
                self.lines.remove(0);
            }
            self.lines.push(Subtitle {
                key: key.to_owned(),
                direction,
                age: 0.0,
            });
        }
        // Newest at the bottom, like the console.
        self.lines.sort_by(|a, b| b.age.total_cmp(&a.age));
    }

    pub fn update(&mut self, seconds: f32) {
        for line in &mut self.lines {
            line.age += seconds;
        }
        self.lines.retain(|line| line.age < LIFETIME);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Oldest first.
    pub fn lines(&self) -> &[Subtitle] {
        &self.lines
    }
}
