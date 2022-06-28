/// Contains the set of vertex and render state data needed to render one in-game frame.
///
/// This data does not contain display lists, but rather the processed display list output.
/// All vertices have already been transformed to clip space.
///
/// This object is self-contained, so all vertex and texture data is stored in the object.
/// Its size is usually around 100-300 KB during normal gameplay.
#[derive(Debug, Clone, Default)]
pub struct SM64RenderData {
    pub(crate) textures: Vec<TextureState>,
    pub(crate) commands: Vec<DrawCommand>,
}

#[derive(Debug, Clone)]
pub(crate) struct TextureState {
    pub(crate) data: Option<TextureData>,
    pub(crate) sampler: Option<SamplerState>,
}

#[derive(Debug, Clone)]
pub(crate) struct TextureData {
    pub(crate) rgba8: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SamplerState {
    pub(crate) linear_filter: bool,
    pub(crate) cms: u32,
    pub(crate) cmt: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct DrawCommand {
    pub(crate) viewport: ScreenRectangle,
    pub(crate) scissor: ScreenRectangle,
    pub(crate) state: RenderState,
    pub(crate) texture_index: [Option<usize>; 2],
    pub(crate) vertex_buffer: Vec<f32>,
    pub(crate) num_tris: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ScreenRectangle {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct RenderState {
    pub(crate) shader_id: Option<u32>,
    pub(crate) depth_test: bool,
    pub(crate) depth_mask: bool,
    pub(crate) zmode_decal: bool,
    pub(crate) use_alpha: bool,
}
