use wafel_data_type::Angle;

#[derive(Debug, Clone, PartialEq)]
pub struct VizConfig {
    pub screen_size: [u32; 2],
    pub camera: Camera,
    pub object_cull: ObjectCull,
    pub elements: Vec<Element>,
}

impl Default for VizConfig {
    fn default() -> Self {
        Self {
            screen_size: [320, 240],
            camera: Default::default(),
            object_cull: Default::default(),
            elements: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectCull {
    Normal,
    ShowAll,
}

impl Default for ObjectCull {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Line(Line),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub vertices: [[f32; 3]; 2],
    pub color: [f32; 4],
}
