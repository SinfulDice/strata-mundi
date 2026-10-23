//! F3 overlay: frame rate, position, loaded world, memory and graphics card, to describe
//! a problem or check performance on the DS without any external tool.

use crate::bench::system::MemorySampler;
use crate::camera::Camera;
use crate::gameplay::Gameplay;
use crate::ui::{Painter, Rect, Theme};
use strata_core::content::Content;
use strata_core::coords::SECTION_SIZE;

/// Frame rate is averaged over this long, so the number is readable.
const FPS_WINDOW_SECONDS: f32 = 0.5;
const MEMORY_EVERY_SECONDS: f32 = 1.0;
const DIRECTIONS: [&str; 4] = ["debug-north", "debug-east", "debug-south", "debug-west"];

#[derive(Default)]
pub struct DebugOverlay {
    pub visible: bool,
    frames: u32,
    elapsed: f32,
    fps: f32,
    frame_ms: f32,
    memory: Option<MemorySampler>,
    since_memory: f32,
}

/// What the overlay describes this frame.
pub struct DebugView<'a> {
    pub content: &'a Content,
    pub language: &'a str,
    pub camera: &'a Camera,
    pub game: Option<&'a Gameplay>,
    pub adapter: &'a wgpu::AdapterInfo,
    pub render_distance: u32,
}

impl DebugOverlay {
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.since_memory = MEMORY_EVERY_SECONDS;
    }

    pub fn record_frame(&mut self, seconds: f32) {
        if !self.visible {
            return;
        }
        self.frames += 1;
        self.elapsed += seconds;
        if self.elapsed >= FPS_WINDOW_SECONDS {
            self.fps = self.frames as f32 / self.elapsed;
            self.frame_ms = self.elapsed * 1000.0 / self.frames as f32;
            (self.frames, self.elapsed) = (0, 0.0);
        }
        self.since_memory += seconds;
        if self.since_memory >= MEMORY_EVERY_SECONDS {
            self.since_memory = 0.0;
            self.memory.get_or_insert_with(MemorySampler::new).sample();
        }
    }

    /// Translated lines, top to bottom; empty when hidden.
    pub fn lines(&self, view: &DebugView<'_>) -> Vec<String> {
        if !self.visible {
            return Vec::new();
        }
        let tr = |key: &str, args: &[(&str, String)]| {
            view.content
                .translations
                .text_with(view.language, key, args)
        };
        let position = view.camera.position;
        let block = position.floor().as_ivec3();
        let yaw = view.camera.yaw.to_degrees();
        let facing = DIRECTIONS[((yaw + 45.0).rem_euclid(360.0) / 90.0) as usize % 4];
        let memory = self.memory.as_ref().map_or(0, |m| m.current_bytes);
        let mut lines = vec![
            tr(
                "debug-title",
                &[("version", env!("CARGO_PKG_VERSION").into())],
            ),
            tr(
                "debug-fps",
                &[
                    ("fps", format!("{:.0}", self.fps)),
                    ("ms", format!("{:.1}", self.frame_ms)),
                ],
            ),
            tr(
                "debug-position",
                &[
                    ("x", format!("{:.2}", position.x)),
                    ("y", format!("{:.2}", position.y)),
                    ("z", format!("{:.2}", position.z)),
                ],
            ),
            tr(
                "debug-section",
                &[
                    ("x", block.x.div_euclid(SECTION_SIZE).to_string()),
                    ("z", block.z.div_euclid(SECTION_SIZE).to_string()),
                    ("y", block.y.div_euclid(SECTION_SIZE).to_string()),
                ],
            ),
            tr(
                "debug-facing",
                &[
                    ("direction", tr(facing, &[])),
                    ("yaw", format!("{yaw:.0}")),
                    ("pitch", format!("{:.0}", view.camera.pitch.to_degrees())),
                ],
            ),
        ];
        if let Some(game) = view.game {
            lines.push(tr(
                "debug-world",
                &[
                    ("columns", game.view.world.column_count().to_string()),
                    ("pending", game.view.pending_meshes().to_string()),
                    ("distance", view.render_distance.to_string()),
                ],
            ));
            if let Some(hit) = game.interaction.target {
                let block = game.view.world.block_at(hit.block);
                let name = view.content.block_name(block);
                lines.push(tr(
                    "debug-target",
                    &[
                        ("block", name.map_or_else(String::new, ToString::to_string)),
                        ("x", hit.block.x.to_string()),
                        ("y", hit.block.y.to_string()),
                        ("z", hit.block.z.to_string()),
                    ],
                ));
            }
        }
        lines.push(tr(
            "debug-memory",
            &[("megabytes", (memory / (1024 * 1024)).to_string())],
        ));
        lines.push(tr(
            "debug-gpu",
            &[
                ("name", view.adapter.name.clone()),
                ("driver", view.adapter.driver.clone()),
            ],
        ));
        lines
    }
}

/// Top-left, one shaded strip per line so the text stays readable over any terrain.
pub fn paint(lines: &[String], painter: &mut Painter<'_>, theme: &Theme) {
    let height = painter.line_height() as i32 + 1;
    for (row, line) in lines.iter().enumerate() {
        let y = 2 + row as i32 * height;
        let width = painter.text_width(line) + 3;
        painter.fill(Rect::new(1, y - 1, width, height as u32), theme.shade.0);
        painter.text(2, y, line, theme.text.0, theme.text_shadow.0);
    }
}

/// Keys the overlay uses, for the missing-translation test.
#[cfg(test)]
pub fn translation_keys() -> Vec<String> {
    [
        "debug-title",
        "debug-fps",
        "debug-position",
        "debug-section",
        "debug-facing",
        "debug-world",
        "debug-target",
        "debug-memory",
        "debug-gpu",
    ]
    .into_iter()
    .chain(DIRECTIONS)
    .map(str::to_owned)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_rate_is_averaged() {
        let mut overlay = DebugOverlay::default();
        overlay.record_frame(0.1);
        assert_eq!(overlay.frames, 0, "hidden: nothing measured");
        overlay.toggle();
        for _ in 0..30 {
            overlay.record_frame(1.0 / 60.0);
        }
        assert!((overlay.fps - 60.0).abs() < 1.0, "{}", overlay.fps);
    }
}
