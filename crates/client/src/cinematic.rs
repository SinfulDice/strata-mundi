//! Plays a pack cinematic: the camera follows the shot, the player stands still, and the
//! view comes back to the player's eyes when it ends or is skipped.

use crate::camera::Camera;
use crate::ui::{Painter, Rect, Theme};
use glam::DVec3;
use strata_core::cinematic::Cinematic;
use strata_core::texture::Rgba;

pub struct PlayingShot {
    shot: Cinematic,
    origin: DVec3,
    time: f32,
    /// Where the player was looking, restored at the end.
    look: (f32, f32),
}

impl PlayingShot {
    pub fn new(shot: Cinematic, origin: DVec3, camera: &Camera) -> Self {
        Self {
            shot,
            origin,
            time: 0.0,
            look: (camera.yaw, camera.pitch),
        }
    }

    /// Moves the camera along the shot; `false` once it is over.
    pub fn advance(&mut self, dt: f32, camera: &mut Camera) -> bool {
        self.time += dt;
        let pose = self.shot.pose(self.time, self.origin);
        camera.position = pose.position;
        camera.yaw = pose
            .yaw_degrees
            .to_radians()
            .rem_euclid(std::f32::consts::TAU);
        camera.pitch = 0.0;
        camera.rotate(0.0, pose.pitch_degrees.to_radians());
        self.time < self.shot.duration()
    }

    pub fn restore(&self, camera: &mut Camera) {
        (camera.yaw, camera.pitch) = self.look;
    }

    pub fn skippable(&self) -> bool {
        self.shot.skippable
    }

    pub fn letterbox(&self) -> bool {
        self.shot.letterbox
    }

    pub fn subtitle(&self) -> Option<&str> {
        self.shot.subtitle(self.time)
    }
}

/// Black bands (film look) and the current subtitle, instead of the HUD.
pub fn paint(
    playing: &PlayingShot,
    subtitle: Option<&str>,
    painter: &mut Painter<'_>,
    screen: Rect,
    theme: &Theme,
) {
    let band = screen.h / 8;
    if playing.letterbox() {
        let black = Rgba([0, 0, 0, 255]);
        painter.fill(Rect::new(0, 0, screen.w, band), black);
        painter.fill(
            Rect::new(0, (screen.h - band) as i32, screen.w, band),
            black,
        );
    }
    if let Some(text) = subtitle {
        let x = (screen.w as i32 - painter.text_width(text) as i32) / 2;
        let y = screen.h as i32 - band as i32 / 2 - painter.line_height() as i32 / 2;
        painter.text(x, y, text, theme.text.0, theme.text_shadow.0);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn the_view_comes_back_after_the_shot() {
        let shot: Cinematic = ron::from_str(
            "(keyframes: [(time: 0.0, pos: (0.0, 5.0, 0.0), yaw: 90.0, pitch: -30.0),
                          (time: 1.0, pos: (0.0, 9.0, 0.0), yaw: 90.0, pitch: -30.0)])",
        )
        .unwrap();
        let mut camera = Camera::new(DVec3::ZERO, 10.0, 5.0, 70.0);
        let mut playing = PlayingShot::new(shot, DVec3::new(1.0, 0.0, 0.0), &camera);
        assert!(playing.advance(0.5, &mut camera));
        assert!((camera.position.y - 7.0).abs() < 0.5 && (camera.position.x - 1.0).abs() < 1e-9);
        assert!(!playing.advance(0.6, &mut camera));
        playing.restore(&mut camera);
        assert!((camera.yaw - 10f32.to_radians()).abs() < 1e-6);
    }
}
