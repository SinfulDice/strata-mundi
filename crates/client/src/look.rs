use crate::camera::Camera;
use crate::input::InputState;

/// Camera rotation from the mouse and the right stick. Values become options at
/// milestone 8.
pub struct LookSettings {
    /// Radians per mouse pixel.
    pub mouse_sensitivity: f32,
    /// Radians per second at full stick deflection.
    pub stick_speed: f32,
}

impl Default for LookSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 0.0025,
            stick_speed: 3.0,
        }
    }
}

impl LookSettings {
    pub fn apply(&self, camera: &mut Camera, input: &InputState, dt: f32) {
        let mouse = input.mouse_delta() * self.mouse_sensitivity;
        let stick = input.look_stick() * self.stick_speed * dt;
        camera.rotate(mouse.x + stick.x, -mouse.y + stick.y);
    }
}
