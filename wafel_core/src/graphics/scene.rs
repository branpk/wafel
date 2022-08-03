//! Representation of the state of the game for the purposes of rendering a game view.

use crate::geo::{direction_to_pitch_yaw, StoredPoint3f, StoredVector3f};
use pyo3::prelude::*;
use std::{collections::HashSet, num::Wrapping};
use wafel_viz::{self as viz, VizConfig};

/// An object representing the state of the game for the purposes of rendering a game view.
///
/// This object is built in Python using Rust helpers, and passed into the Rust renderer.
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct Scene {
    /// The viewport in screen coordinates where the scene should be rendered.
    #[pyo3(get, set)]
    pub viewport: Viewport,
    /// The bird's eye or rotational camera to use.
    pub camera: Camera,
    /// Whether to draw the camera's focus point (RotateCamera only).
    #[pyo3(get, set)]
    pub show_camera_target: bool,
    /// The SM64 surfaces to render.
    pub surfaces: Vec<Surface>,
    /// The size of wall hitboxes. Setting this to 0 disables them.
    #[pyo3(get, set)]
    pub wall_hitbox_radius: f32,
    /// The surface to highlight because the mouse cursor is above it (index into `surfaces`).
    #[pyo3(get, set)]
    pub hovered_surface: Option<usize>,
    /// The surfaces that have been hidden by the user (indices into `surfaces`).
    #[pyo3(get, set)]
    pub hidden_surfaces: HashSet<usize>,
    /// The SM64 objects to render.
    pub objects: Vec<Object>,
    /// The detailed paths of object movement across multiple frames.
    #[pyo3(get, set)]
    pub object_paths: Vec<ObjectPath>,
}

impl Scene {
    /// Return a [VizConfig] to use when rendering the scene using [wafel_viz].
    pub fn to_viz_config(&self) -> VizConfig {
        let screen_size = [self.viewport.width as u32, self.viewport.height as u32];
        let camera = match &self.camera {
            Camera::Rotate(camera) => viz::Camera::LookAt {
                pos: [camera.pos.x, camera.pos.y, camera.pos.z],
                focus: [camera.target.x, camera.target.y, camera.target.z],
                roll: Wrapping(0),
            },
            Camera::BirdsEye(_) => unimplemented!(),
        };
        VizConfig {
            screen_size,
            camera,
            object_cull: viz::ObjectCull::ShowAll,
            ..Default::default()
        }
    }
}

#[pymethods]
impl Scene {
    /// Create an empty scene.
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the camera as a python object.
    #[getter]
    pub fn get_camera(&self, py: Python<'_>) -> PyObject {
        match self.camera.clone() {
            Camera::Rotate(camera) => camera.into_py(py),
            Camera::BirdsEye(camera) => camera.into_py(py),
        }
    }

    /// Set the camera from a python object.
    #[setter]
    pub fn set_camera(&mut self, camera: &PyAny) -> PyResult<()> {
        if let Ok(rotate_camera) = camera.extract::<RotateCamera>() {
            self.camera = Camera::Rotate(rotate_camera);
        } else {
            self.camera = Camera::BirdsEye(camera.extract()?);
        }
        Ok(())
    }
}

/// A rectangular viewport in screen coordinates.
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct Viewport {
    /// The left x coordinate.
    #[pyo3(get, set)]
    pub x: f32,
    /// The upper y coordinate.
    #[pyo3(get, set)]
    pub y: f32,
    /// The width in pixels.
    #[pyo3(get, set)]
    pub width: f32,
    /// The height in pixels.
    #[pyo3(get, set)]
    pub height: f32,
}

#[pymethods]
impl Viewport {
    /// Create a viewport with zero values.
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A camera used to render a scene.
#[derive(Debug, Clone)]
pub enum Camera {
    /// A 3D rotational camera.
    Rotate(RotateCamera),
    /// A 2D overhead view.
    BirdsEye(BirdsEyeCamera),
}

impl Default for Camera {
    fn default() -> Self {
        Self::BirdsEye(BirdsEyeCamera::default())
    }
}

/// A 3D rotational camera.
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct RotateCamera {
    /// The position of the camera.
    #[pyo3(get, set)]
    pub pos: StoredPoint3f,
    /// The point that the camera is focusing on.
    ///
    /// This is rendered if `show_camera_target` is enabled.
    #[pyo3(get, set)]
    pub target: StoredPoint3f,
    /// The y FOV in radians.
    #[pyo3(get, set)]
    pub fov_y: f32,
}

#[pymethods]
impl RotateCamera {
    /// Create a new camera with zero values.
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the camera's facing pitch.
    #[getter]
    pub fn pitch(&self) -> f32 {
        direction_to_pitch_yaw(&(self.target.0 - self.pos.0)).0
    }

    /// Get the camera's facing yaw.
    #[getter]
    pub fn yaw(&self) -> f32 {
        direction_to_pitch_yaw(&(self.target.0 - self.pos.0)).1
    }
}

/// A 2D overhead view camera.
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct BirdsEyeCamera {
    /// The position of the camera.
    ///
    /// Only objects below this point will be rendered.
    #[pyo3(get, set)]
    pub pos: StoredPoint3f,
    /// The world space distance covered by the vertical span of the viewport.
    #[pyo3(get, set)]
    pub span_y: f32,
}

#[pymethods]
impl BirdsEyeCamera {
    /// Create a camera with zero values.
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

/// SM64 surface data used for rendering.
#[derive(Debug, Clone)]
pub struct Surface {
    /// The type of the surface.
    pub ty: SurfaceType,
    /// The surface's vertices.
    pub vertices: [StoredPoint3f; 3],
    /// The surface's normal vector.
    pub normal: StoredVector3f,
}

/// An SM64 surface type, based on the game's definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceType {
    /// Normal is pointing somewhat upward.
    Floor,
    /// Normal is pointing somewhat downward.
    Ceiling,
    /// Normal is almost horizontal and points more in the X direction.
    WallXProj,
    /// Normal is almost horizontal and points more in the Z direction.
    WallZProj,
}

/// SM64 object data used for rendering.
#[derive(Debug, Clone)]
pub struct Object {
    /// The position of the object.
    pub pos: StoredPoint3f,
    /// The physical hitbox height of the object.
    pub hitbox_height: f32,
    /// The physical hitbox radius of the object.
    pub hitbox_radius: f32,
}

/// Info about an object's movement over the course of several frames.
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct ObjectPath {
    /// The list of nodes along the path, in chronological order.
    pub nodes: Vec<ObjectPathNode>,
    /// The index into `nodes` that is considered the focus (i.e. the currently selected frame).
    #[pyo3(get, set)]
    pub root_index: usize,
}

#[pymethods]
impl ObjectPath {
    /// Convenience method to set the quarter steps for a single node.
    pub fn set_quarter_steps(&mut self, index: usize, quarter_steps: Vec<QuarterStep>) {
        self.nodes[index].quarter_steps = quarter_steps;
    }
}

/// A single node in an object path, with information for a single frame.
#[derive(Debug, Clone, Default)]
pub struct ObjectPathNode {
    /// The object's position at the start of the frame.
    pub pos: StoredPoint3f,
    /// The quarter steps that occurred during the frame (i.e. leading out of `pos`).
    ///
    /// This list may be empty or have fewer than 4 elements.
    pub quarter_steps: Vec<QuarterStep>,
}

/// A single quarter step within a frame.
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct QuarterStep {
    /// The intended position of the step before surface collisions were resolved.
    #[pyo3(get, set)]
    pub intended_pos: StoredPoint3f,
    /// The final position of the step.
    #[pyo3(get, set)]
    pub result_pos: StoredPoint3f,
}

#[pymethods]
impl QuarterStep {
    /// Create an empty quarter step node.
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}
