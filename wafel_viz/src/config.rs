use wafel_data_type::Angle;

#[derive(Debug, Clone, PartialEq)]
pub struct SM64RenderConfig {
    pub screen_size: [u32; 2],
    pub camera: Camera,
    pub object_cull: ObjectCull,
}

impl Default for SM64RenderConfig {
    fn default() -> Self {
        Self {
            screen_size: [320, 240],
            camera: Default::default(),
            object_cull: Default::default(),
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
