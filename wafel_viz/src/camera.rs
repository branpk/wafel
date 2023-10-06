use ultraviolet::{projection::rh_yup::perspective_wgpu_dx, Mat4, Vec3, Vec4};

/// Defines the projection and view matrices for a scene.
///
/// We assume that NDC space is `[-1, 1]` in x and y and `[0, 1]` in z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// The projection matrix from view space to NDC.
    pub proj_mtx: Mat4,
    /// The view matrix from world space to view space.
    pub view_mtx: Mat4,
}

impl Camera {
    /// Creates a new camera with the given projection and view matrices.
    pub fn new(proj_mtx: Mat4, view_mtx: Mat4) -> Self {
        Self { proj_mtx, view_mtx }
    }

    /// Creates a perspective camera, which by default looks from the origin
    /// down the negative z axis, from z = `-near` to z = `-far`.
    pub fn perspective(fov_y_degrees: f32, aspect: f32, near: f32, far: f32) -> Self {
        assert!(fov_y_degrees > 0.0);
        assert!(aspect > 0.0);
        assert!(near > 0.0);
        assert!(far > near);

        Self {
            proj_mtx: perspective_wgpu_dx(fov_y_degrees.to_radians(), aspect, near, far),
            view_mtx: Mat4::identity(),
        }
    }

    /// Creates an orthographic camera, which by default looks down the negative
    /// z axis, from z = `-near` to z = `-far`.
    ///
    /// In world coordinates, the view ranges from `(-span_x/2, -span_y/2)`
    /// to `(span_x/2, span_y/2)`.
    pub fn orthographic(span_x: f32, span_y: f32, near: f32, far: f32) -> Self {
        assert!(span_x > 0.0);
        assert!(span_y > 0.0);
        assert!(far > near);

        let span_z = far - near;
        let scale =
            Mat4::from_nonuniform_scale(Vec3::new(2.0 / span_x, 2.0 / span_y, -1.0 / span_z));
        let translate = Mat4::from_translation(Vec3::new(0.0, 0.0, near));
        Self {
            proj_mtx: scale * translate,
            view_mtx: Mat4::identity(),
        }
    }

    /// Translate the camera's position.
    pub fn translate(mut self, disp: Vec3) -> Self {
        self.view_mtx = Mat4::from_translation(-disp) * self.view_mtx;
        self
    }

    /// Rotate the camera's facing angle without changing its position.
    pub fn rotate(mut self, roll: f32, pitch: f32, yaw: f32) -> Self {
        self.view_mtx = Mat4::from_euler_angles(-roll, -pitch, -yaw) * self.view_mtx;
        self
    }

    /// Moves the camera to the given position and faces it toward the given point.
    pub fn look_at(self, camera_pos: Vec3, focus: Vec3) -> Self {
        self.look_at_with_roll(camera_pos, focus, 0.0)
    }

    /// Moves the camera to the given position and faces it toward the given point,
    pub fn look_at_with_roll(mut self, camera_pos: Vec3, focus: Vec3, roll: f32) -> Self {
        let translation = Mat4::from_translation(-camera_pos);

        let mut forward = focus - camera_pos;
        let mag = forward.mag();
        if mag < 0.0001 {
            self.view_mtx = translation;
            return self;
        }
        forward /= mag;

        let mut rightward = forward.cross(Vec3::unit_y());
        let mag = rightward.mag();
        if mag < 0.0001 {
            rightward = Vec3::unit_x();
        }
        rightward /= rightward.mag();

        let upward = rightward.cross(forward);

        let focus_rotation = Mat4::new(
            rightward.into_homogeneous_vector(),
            upward.into_homogeneous_vector(),
            -forward.into_homogeneous_vector(),
            Vec4::unit_w(),
        )
        .transposed();

        let roll_rotation = Mat4::from_euler_angles(0.0, 0.0, -roll);

        self.view_mtx = roll_rotation * focus_rotation * translation;
        self
    }
}
