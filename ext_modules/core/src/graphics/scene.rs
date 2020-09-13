use crate::geo::{Point3f, Vector3f};
use pyo3::prelude::*;
use std::collections::HashSet;

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct Scene {
    #[pyo3(get, set)]
    pub viewport: Viewport,
    pub camera: Camera,
    #[pyo3(get, set)]
    pub show_camera_target: bool,
    pub surfaces: Vec<Surface>,
    #[pyo3(get, set)]
    pub wall_hitbox_radius: f32,
    #[pyo3(get, set)]
    pub hovered_surface: Option<usize>,
    #[pyo3(get, set)]
    pub hidden_surfaces: HashSet<usize>,
    pub objects: Vec<Object>,
    #[pyo3(get, set)]
    pub object_paths: Vec<ObjectPath>,
}

#[pymethods]
impl Scene {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    #[getter]
    pub fn get_camera(&self, py: Python<'_>) -> PyObject {
        match self.camera.clone() {
            Camera::Rotate(camera) => camera.into_py(py),
            Camera::BirdsEye(camera) => camera.into_py(py),
        }
    }

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

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct Viewport {
    #[pyo3(get, set)]
    pub x: f32,
    #[pyo3(get, set)]
    pub y: f32,
    #[pyo3(get, set)]
    pub width: f32,
    #[pyo3(get, set)]
    pub height: f32,
}

#[pymethods]
impl Viewport {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub enum Camera {
    Rotate(RotateCamera),
    BirdsEye(BirdsEyeCamera),
}

impl Default for Camera {
    fn default() -> Self {
        Self::BirdsEye(BirdsEyeCamera::default())
    }
}

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct RotateCamera {
    #[pyo3(get, set)]
    pub pos: [f32; 3],
    #[pyo3(get, set)]
    pub target: [f32; 3],
    #[pyo3(get, set)]
    pub fov_y: f32,
}

#[pymethods]
impl RotateCamera {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct BirdsEyeCamera {
    #[pyo3(get, set)]
    pub pos: [f32; 3],
    #[pyo3(get, set)]
    pub span_y: f32,
}

#[pymethods]
impl BirdsEyeCamera {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub struct Surface {
    pub ty: SurfaceType,
    pub vertices: [[f32; 3]; 3],
    pub normal: [f32; 3],
}

impl Surface {
    pub fn normal(&self) -> Vector3f {
        Vector3f::from_row_slice(&self.normal)
    }

    pub fn vertices(&self) -> [Point3f; 3] {
        [
            Point3f::from_slice(&self.vertices[0]),
            Point3f::from_slice(&self.vertices[1]),
            Point3f::from_slice(&self.vertices[2]),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SurfaceType {
    Floor,
    Ceiling,
    WallXProj,
    WallZProj,
}

#[derive(Debug, Clone)]
pub struct Object {
    pub pos: [f32; 3],
    pub hitbox_height: f32,
    pub hitbox_radius: f32,
}

impl Object {
    pub fn pos(&self) -> Point3f {
        Point3f::from_slice(&self.pos)
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ObjectPath {
    pub nodes: Vec<ObjectPathNode>,
    #[pyo3(get, set)]
    pub root_index: usize,
}

#[pymethods]
impl ObjectPath {
    pub fn set_quarter_steps(&mut self, index: usize, quarter_steps: Vec<QuarterStep>) {
        self.nodes[index].quarter_steps = quarter_steps;
    }
}

#[derive(Debug, Clone)]
pub struct ObjectPathNode {
    pub pos: [f32; 3],
    pub quarter_steps: Vec<QuarterStep>,
}

impl ObjectPathNode {
    pub fn pos(&self) -> Point3f {
        Point3f::from_slice(&self.pos)
    }
}

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct QuarterStep {
    #[pyo3(get, set)]
    pub intended_pos: [f32; 3],
    #[pyo3(get, set)]
    pub result_pos: [f32; 3],
}

#[pymethods]
impl QuarterStep {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }
}
