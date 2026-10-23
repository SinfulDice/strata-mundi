use glam::camera::rh::{proj::directx, view};
use glam::{DVec3, Mat4, Vec3};

/// Free camera. World position in `f64`; rendering is relative to the camera so that
/// precision stays perfect even millions of blocks away from the origin.
#[derive(Debug, Clone, PartialEq)]
pub struct Camera {
    pub position: DVec3,
    /// Radians. 0 looks toward -Z (north), increasing toward +X (east).
    pub yaw: f32,
    /// Radians. Positive looks up.
    pub pitch: f32,
    pub fov_y_degrees: f32,
}

const MAX_PITCH: f32 = 89.9_f32 * std::f32::consts::PI / 180.0;
const NEAR_PLANE: f32 = 0.05;

impl Camera {
    pub fn new(position: DVec3, yaw_degrees: f32, pitch_degrees: f32, fov_y_degrees: f32) -> Self {
        let mut camera = Self {
            position,
            yaw: yaw_degrees.to_radians(),
            pitch: 0.0,
            fov_y_degrees,
        };
        camera.rotate(0.0, pitch_degrees.to_radians());
        camera
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw = (self.yaw + delta_yaw).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + delta_pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    pub fn forward(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        Vec3::new(cos_pitch * sin_yaw, sin_pitch, -cos_pitch * cos_yaw)
    }

    /// Horizontal forward, ignoring pitch (movement stays level when looking down).
    pub fn flat_forward(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(sin_yaw, 0.0, -cos_yaw)
    }

    pub fn flat_right(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        Vec3::new(cos_yaw, 0.0, sin_yaw)
    }

    /// View-projection with the camera at the origin, reverse-Z and an infinite far plane.
    /// wgpu uses the DirectX clip-space convention (depth 0..1, Y up) on every backend.
    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        let view = view::look_to_mat4(Vec3::ZERO, self.forward(), Vec3::Y);
        let projection = directx::perspective_infinite_reverse(
            self.fov_y_degrees.to_radians(),
            aspect_ratio.max(0.01),
            NEAR_PLANE,
        );
        projection * view
    }

    /// Converts a world position to camera-relative coordinates for the GPU.
    pub fn relative(&self, world: DVec3) -> Vec3 {
        (world - self.position).as_vec3()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn yaw_zero_looks_north_and_ninety_looks_east() {
        let north = Camera::new(DVec3::ZERO, 0.0, 0.0, 70.0);
        assert!(close(north.forward(), Vec3::NEG_Z));
        assert!(close(north.flat_right(), Vec3::X));
        let east = Camera::new(DVec3::ZERO, 90.0, 0.0, 70.0);
        assert!(close(east.forward(), Vec3::X));
    }

    #[test]
    fn pitch_is_clamped() {
        let mut camera = Camera::new(DVec3::ZERO, 0.0, 0.0, 70.0);
        camera.rotate(0.0, 10.0);
        assert!(camera.forward().y > 0.999 && camera.forward().y < 1.0);
    }

    #[test]
    fn relative_position_is_precise_far_from_origin() {
        let camera = Camera::new(
            DVec3::new(9_999_999.5, 100.0, -9_999_999.25),
            0.0,
            0.0,
            70.0,
        );
        let relative = camera.relative(DVec3::new(10_000_000.0, 101.0, -10_000_000.0));
        assert!(close(relative, Vec3::new(0.5, 1.0, -0.75)));
    }
}
