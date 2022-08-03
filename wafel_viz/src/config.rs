use serde::{Deserialize, Serialize};
use wafel_data_type::Angle;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VizConfig {
    pub screen_top_left: [u32; 2],
    pub screen_size: [u32; 2],
    pub camera: Camera,
    pub object_cull: ObjectCull,
    pub surface_mode: SurfaceMode,
    pub elements: Vec<Element>,
}

impl Default for VizConfig {
    fn default() -> Self {
        Self {
            screen_top_left: [0, 0],
            screen_size: [320, 240],
            camera: Default::default(),
            object_cull: Default::default(),
            surface_mode: Default::default(),
            elements: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Camera {
    InGame,
    LookAt {
        pos: [f32; 3],
        focus: [f32; 3],
        roll: Angle,
    },
}

impl Default for Camera {
    fn default() -> Self {
        Self::InGame
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectCull {
    Normal,
    ShowAll,
}

impl Default for ObjectCull {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfaceMode {
    Visual,
    Physical,
    None,
}

impl Default for SurfaceMode {
    fn default() -> Self {
        Self::Visual
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Element {
    Line(Line),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub vertices: [[f32; 3]; 2],
    pub color: [f32; 4],
}
