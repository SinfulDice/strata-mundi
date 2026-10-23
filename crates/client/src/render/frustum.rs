use glam::{Mat4, Vec3, Vec4};

/// Side and near planes of the view volume, in camera-relative space (the far plane is
/// at infinity). A box entirely outside one plane is not drawn.
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Vec4; 5],
}

impl Frustum {
    /// Gribb–Hartmann extraction from a view-projection matrix with depth in `0..1`.
    pub fn from_view_projection(matrix: Mat4) -> Self {
        let rows = [matrix.row(0), matrix.row(1), matrix.row(2), matrix.row(3)];
        let planes = [
            rows[3] + rows[0],
            rows[3] - rows[0],
            rows[3] + rows[1],
            rows[3] - rows[1],
            // Reverse-Z: the near plane is where clip depth equals w.
            rows[3] - rows[2],
        ]
        .map(|plane| plane / plane.truncate().length().max(f32::EPSILON));
        Self { planes }
    }

    /// Axis-aligned box from `min` to `max`, camera-relative.
    pub fn intersects_box(&self, min: Vec3, max: Vec3) -> bool {
        self.planes.iter().all(|plane| {
            let normal = plane.truncate();
            let farthest = Vec3::select(normal.cmpge(Vec3::ZERO), max, min);
            normal.dot(farthest) + plane.w >= 0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use glam::DVec3;

    fn frustum() -> Frustum {
        let camera = Camera::new(DVec3::ZERO, 0.0, 0.0, 70.0);
        Frustum::from_view_projection(camera.view_projection(16.0 / 9.0))
    }

    #[test]
    fn boxes_ahead_are_visible_and_behind_are_not() {
        let frustum = frustum();
        let unit = Vec3::ONE;
        assert!(frustum.intersects_box(
            Vec3::new(-0.5, -0.5, -20.0),
            Vec3::new(-0.5, -0.5, -20.0) + unit
        ));
        assert!(!frustum.intersects_box(
            Vec3::new(-0.5, -0.5, 20.0),
            Vec3::new(-0.5, -0.5, 20.0) + unit
        ));
        assert!(!frustum.intersects_box(Vec3::new(100.0, 0.0, -5.0), Vec3::new(101.0, 1.0, -4.0)));
    }

    #[test]
    fn box_around_the_camera_is_visible() {
        assert!(frustum().intersects_box(Vec3::splat(-8.0), Vec3::splat(8.0)));
    }

    #[test]
    fn very_far_boxes_ahead_stay_visible() {
        let min = Vec3::new(-1.0, -1.0, -5000.0);
        assert!(frustum().intersects_box(min, min + Vec3::splat(16.0)));
    }
}
