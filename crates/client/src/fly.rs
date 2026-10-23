use crate::camera::Camera;
use crate::input::{Action, InputState};
use glam::{DVec3, Vec3};

/// Free flight used until the player entity exists (milestone 6).
pub struct FlyController {
    /// Blocks per second.
    pub speed: f32,
    pub sprint_multiplier: f32,
    /// Radians per mouse pixel.
    pub mouse_sensitivity: f32,
    /// Radians per second at full stick deflection.
    pub stick_look_speed: f32,
}

impl Default for FlyController {
    fn default() -> Self {
        Self {
            speed: 10.9,
            sprint_multiplier: 2.0,
            mouse_sensitivity: 0.0025,
            stick_look_speed: 3.0,
        }
    }
}

impl FlyController {
    pub fn update(&self, camera: &mut Camera, input: &InputState, dt: f32) {
        let mouse = input.mouse_delta() * self.mouse_sensitivity;
        let stick = input.look_stick() * self.stick_look_speed * dt;
        camera.rotate(mouse.x + stick.x, -mouse.y + stick.y);

        let wish = input.movement();
        let vertical = f32::from(u8::from(input.is_held(Action::Jump)))
            - f32::from(u8::from(input.is_held(Action::Sneak)));
        let direction =
            camera.flat_right() * wish.x + camera.flat_forward() * wish.y + Vec3::Y * vertical;
        let speed = if input.is_held(Action::Sprint) {
            self.speed * self.sprint_multiplier
        } else {
            self.speed
        };
        camera.position += DVec3::from(direction.clamp_length_max(1.0) * speed * dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Binding, Bindings};
    use winit::keyboard::KeyCode;

    #[test]
    fn forward_follows_yaw_and_ignores_pitch() {
        let mut camera = Camera::new(DVec3::ZERO, 90.0, -60.0, 70.0);
        let mut input = InputState::new(Bindings::default());
        input.set_binding(Binding::Key(KeyCode::KeyW), true);
        FlyController::default().update(&mut camera, &input, 1.0);
        assert!((camera.position - DVec3::new(10.9, 0.0, 0.0)).length() < 1e-4);
    }
}
