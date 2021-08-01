//! Geometric types and functions.

use std::ops::Deref;

use bytemuck::{Pod, Zeroable};
use pyo3::{types::PyTuple, FromPyObject, IntoPy, PyObject};

/// 4x4 f32 matrix
pub type Matrix4f = nalgebra::Matrix4<f32>;
/// f32 vector of length 3
pub type Vector3f = nalgebra::Vector3<f32>;
/// f32 vector of length 4
pub type Vector4f = nalgebra::Vector4<f32>;
/// f32 point of length 3
pub type Point3f = nalgebra::Point3<f32>;

macro_rules! stored_matrix_wrapper {
    ($name:ident, $ty:ty) => {
        /// Wrapper type, mainly for Pod and pyo3 conversion implementations.
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct $name(pub $ty);

        unsafe impl Zeroable for $name {}
        unsafe impl Pod for $name {}

        impl From<$ty> for $name {
            fn from(x: $ty) -> Self {
                Self(x)
            }
        }

        impl Deref for $name {
            type Target = $ty;

            fn deref(&self) -> &$ty {
                &self.0
            }
        }
    };
}

stored_matrix_wrapper!(StoredMatrix4f, Matrix4f);
stored_matrix_wrapper!(StoredVector3f, Vector3f);
stored_matrix_wrapper!(StoredVector4f, Vector4f);

/// Wrapper type, mainly for Pod and pyo3 conversion implementations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StoredPoint3f(pub Point3f);

unsafe impl Zeroable for StoredPoint3f {}
unsafe impl Pod for StoredPoint3f {}

impl From<Point3f> for StoredPoint3f {
    fn from(point: Point3f) -> Self {
        Self(point)
    }
}

impl From<[f32; 3]> for StoredPoint3f {
    fn from(v: [f32; 3]) -> Self {
        Self(v.into())
    }
}

impl Deref for StoredPoint3f {
    type Target = Point3f;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for StoredPoint3f {
    fn default() -> Self {
        Self(Point3f::origin())
    }
}

impl<'p> FromPyObject<'p> for StoredPoint3f {
    fn extract(object: &'p pyo3::PyAny) -> pyo3::PyResult<Self> {
        let coords: [f32; 3] = object.extract()?;
        Ok(Self(Point3f::new(coords[0], coords[1], coords[2])))
    }
}

impl IntoPy<PyObject> for StoredPoint3f {
    fn into_py(self, py: pyo3::Python<'_>) -> PyObject {
        PyTuple::new(py, &[self.x, self.y, self.z]).into_py(py)
    }
}

/// Convert a direction to its pitch and yaw in radians.
pub fn direction_to_pitch_yaw(dir: &Vector3f) -> (f32, f32) {
    let xz = (dir.x * dir.x + dir.z * dir.z).sqrt();
    let pitch = f32::atan2(dir.y, xz);
    let yaw = f32::atan2(dir.x, dir.z);
    (pitch, yaw)
}

/// Convert a pitch and yaw in radians to a direction.
pub fn pitch_yaw_to_direction(pitch: f32, yaw: f32) -> Vector3f {
    Vector3f::new(
        pitch.cos() * yaw.sin(),
        pitch.sin(),
        pitch.cos() * yaw.cos(),
    )
}
