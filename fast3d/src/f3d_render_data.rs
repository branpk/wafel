use std::collections::HashMap;

/// An object containing processed render data from a display list.
///
/// Vertices are already transformed to screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct F3DRenderData {
    /// The screen size in pixels.
    pub screen_size: [u32; 2],
    /// Pipeline modes that are used by the commands.
    pub pipelines: HashMap<PipelineId, PipelineInfo>,
    /// Textures that are used by the commands.
    pub textures: HashMap<TextureIndex, TextureState>,
    /// The draw calls to issue.
    pub commands: Vec<DrawCommand<Vec<f32>>>,
}

impl F3DRenderData {
    /// Create an empty F3DRenderData.
    pub fn new(screen_size: [u32; 2]) -> Self {
        Self {
            screen_size,
            pipelines: HashMap::new(),
            textures: HashMap::new(),
            commands: Vec::new(),
        }
    }
}

/// An id for the pipeline state of a draw call.
///
/// There is a global 1-1 correspondance between ids and [PipelineInfo]s such that pipelines
/// can be precomputed or cached across frames.
// TODO: Pack pipeline info to get id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineId(pub(crate) PipelineInfo);

/// The pipeline state used for a draw call.
///
/// The vertex attributes are:
/// - vec4 pos
/// - if uses_textures(): vec2 uv
/// - if fog: vec4 fog
/// - for i in 0..num_inputs: vec4 input_i
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PipelineInfo {
    /// Face cull mode.
    pub cull_mode: CullMode,
    /// Enable standard depth buffer compare.
    pub depth_compare: bool,
    /// Enable updates to the depth buffer.
    pub depth_write: bool,
    /// Use standard alpha blending.
    pub blend: bool,
    /// We are drawing a decal, so z values should be offset to avoid artifacts.
    pub decal: bool,
    /// Which texture bindings should be enabled.
    /// [DrawCommand] specifies which textures to bind.
    pub used_textures: [bool; 2],
    /// Uses an alpha edge mask, so fragments with low alpha should be discarded to avoid
    /// edge blur.
    pub texture_edge: bool,
    /// Enable fog. This is calculated as mix(color, fog.rgb, fog.a).
    pub fog: bool,
    /// Enable random dithering (e.g. SM64 vanish cap effect).
    pub noise: bool,
    /// The number of vec4 inputs after pos/uv/fog.
    pub num_inputs: u32,
    /// Expression for calculating fragment output color.
    pub output_color: ColorExpr,
}

impl PipelineInfo {
    /// Returns true if either texture is used.
    pub fn uses_textures(&self) -> bool {
        self.used_textures[0] || self.used_textures[1]
    }
}

#[allow(missing_docs)]
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

/// An expression for the fragment color output.
///
/// Each field is an array with four per-fragment values. Each element
/// corresponds to a vector of length 4, and they are combined as follows:
///
/// [a, b, c, d] -> (a - b) * c + d
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorExpr {
    /// Expression for calculating rgb. (Truncate to vec3 after evaluating the vec4 expression.)
    pub rgb: [ColorArg; 4],
    /// Expression for calculating alpha. (Take .a after evaluating the vec4 expression.)
    /// If blending is disabled, then alpha = 1 should be used instead.
    pub a: [ColorArg; 4],
}

/// An argument to [ColorExpr].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorArg {
    /// (0, 0, 0, 0)
    Zero,
    /// The i-th vertex input (after pos/uv/fog).
    Input(u32),
    /// The rgba value from texture 0.
    Texel0,
    /// The alpha value from texture 0, repeated to a vec4.
    Texel0Alpha,
    /// The rgba value from texture 1.
    Texel1,
}

impl Default for ColorArg {
    fn default() -> Self {
        Self::Zero
    }
}

/// An id for a loaded texture.
///
/// Unlike [PipelineId], these may be reused across frames so textures shouldn't be
/// cached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureIndex(pub(crate) u32);

/// Texture pixel data and sampling parameters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TextureState {
    #[allow(missing_docs)]
    pub data: TextureData,
    #[allow(missing_docs)]
    pub sampler: SamplerState,
}

/// Rgba8 texture buffer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TextureData {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// Rgba8 color data (32 bits per pixel).
    pub rgba8: Vec<u8>,
}

/// Sampling parameters for a texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SamplerState {
    #[allow(missing_docs)]
    pub u_wrap: WrapMode,
    #[allow(missing_docs)]
    pub v_wrap: WrapMode,
    #[allow(missing_docs)]
    pub linear_filter: bool,
}

#[allow(missing_docs)]
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

/// One triangle list draw call.
///
/// See [PipelineInfo] for details.
#[allow(missing_docs)]
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
    #[allow(missing_docs)]
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

/// A rectangle in screen space.
///
/// The upper left corner is (0, 0) and the lower right corner is (screen width, screen height).
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ScreenRectangle {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
