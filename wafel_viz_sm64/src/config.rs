use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use wafel_data_type::Angle;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VizConfig {
    pub screen_top_left: [i32; 2],
    pub screen_size: [i32; 2],
    pub in_game_render_mode: InGameRenderMode,
    pub show_in_game_overlays: bool,
    pub camera: Camera,
    pub show_camera_focus: bool,
    pub object_cull: ObjectCull,
    pub surface_mode: SurfaceMode,
    pub wall_hitbox_radius: f32,
    pub highlighted_surfaces: HashSet<usize>,
    pub transparent_surfaces: HashSet<usize>,
    pub elements: Vec<Element>,
}

impl Default for VizConfig {
    fn default() -> Self {
        Self {
            screen_top_left: [0, 0],
            screen_size: [320, 240],
            in_game_render_mode: InGameRenderMode::default(),
            show_in_game_overlays: true,
            camera: Camera::default(),
            show_camera_focus: false,
            object_cull: ObjectCull::default(),
            surface_mode: SurfaceMode::default(),
            wall_hitbox_radius: 0.0,
            highlighted_surfaces: HashSet::new(),
            transparent_surfaces: HashSet::new(),
            elements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InGameRenderMode {
    Rerender,
    DisplayList,
    Disabled,
}

impl Default for InGameRenderMode {
    fn default() -> Self {
        Self::Rerender
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Camera {
    InGame,
    LookAt(LookAtCamera),
    Ortho(OrthoCamera),
}

impl Default for Camera {
    fn default() -> Self {
        Self::InGame
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LookAtCamera {
    pub pos: [f32; 3],
    pub focus: [f32; 3],
    pub roll: Angle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrthoCamera {
    pub pos: [f32; 3],
    pub forward: [f32; 3],
    pub upward: [f32; 3],
    pub span_v: f32,
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
    Point(Point),
    Line(Line),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub pos: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub vertices: [[f32; 3]; 2],
    pub color: [f32; 4],
}
