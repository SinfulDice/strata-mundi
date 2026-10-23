use crate::camera::Camera;
use crate::input::InputState;
use crate::settings::ControlSettings;

/// Turns the camera with the mouse and the right stick, as set in the options.
pub fn apply_look(camera: &mut Camera, input: &InputState, dt: f32, controls: &ControlSettings) {
    let mouse = input.mouse_delta() * controls.mouse_radians();
    let stick = input.look_stick() * controls.stick_radians() * dt;
    let vertical = -mouse.y + stick.y;
    let vertical = if controls.invert_y {
        -vertical
    } else {
        vertical
    };
    camera.rotate(mouse.x + stick.x, vertical);
}
