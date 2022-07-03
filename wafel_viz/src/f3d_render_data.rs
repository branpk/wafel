use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct F3DRenderData {
    pub pipelines: HashMap<PipelineId, PipelineInfo>,
    pub textures: HashMap<TextureIndex, TextureState>,
    pub commands: Vec<DrawCommand<Vec<f32>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId {
    pub(crate) state: PipelineInfo, // TODO: Convert to int and add derive Copy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PipelineInfo {
    pub cull_mode: CullMode,
    pub depth_compare: bool,
    pub depth_write: bool,
    pub blend: bool,
    pub decal: bool,
    pub used_textures: [bool; 2],
    pub texture_edge: bool,
    pub fog: bool,
    pub num_inputs: u32,
    pub output_color: ColorExpr,
}

impl PipelineInfo {
    pub fn uses_textures(&self) -> bool {
        self.used_textures[0] || self.used_textures[1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CullMode {
    None,
    Front,
    Back,
}

impl Default for CullMode {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorExpr {
    pub rgb: [ColorArg; 4],
    pub a: [ColorArg; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorArg {
    Zero,
    Input(u32),
    Texel0,
    Texel0Alpha,
    Texel1,
}

impl Default for ColorArg {
    fn default() -> Self {
        Self::Zero
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureIndex(pub(crate) u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TextureState {
    pub data: TextureData,
    pub sampler: SamplerState,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SamplerState {
    pub u_wrap: WrapMode,
    pub v_wrap: WrapMode,
    pub linear_filter: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapMode {
    Clamp,
    Repeat,
    MirrorRepeat,
}

impl Default for WrapMode {
    fn default() -> Self {
        Self::Repeat
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DrawCommand<B> {
    pub viewport: ScreenRectangle,
    pub scissor: ScreenRectangle,
    pub pipeline: PipelineId,
    pub textures: [Option<TextureIndex>; 2],
    pub vertex_buffer: B,
    pub num_vertices: u32,
}

impl<B> DrawCommand<B> {
    pub fn with_buffer<T>(&self, buffer: T) -> DrawCommand<T> {
        DrawCommand {
            viewport: self.viewport,
            scissor: self.scissor,
            pipeline: self.pipeline,
            textures: self.textures,
            vertex_buffer: buffer,
            num_vertices: self.num_vertices,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ScreenRectangle {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
