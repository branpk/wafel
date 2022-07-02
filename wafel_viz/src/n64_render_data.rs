use core::fmt;

use crate::render_api::CullMode;

/// Contains the set of vertex and render state data needed to render one in-game frame.
///
/// This data does not contain display lists, but rather the processed display list output.
/// All vertices have already been transformed to clip space.
///
/// This object is self-contained, so all vertex and texture data is stored in the object.
/// Its size is usually around 100-300 KB during normal gameplay.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct N64RenderData {
    pub(crate) textures: Vec<TextureState>,
    pub(crate) commands: Vec<DrawCommand>,
}

impl N64RenderData {
    pub fn compare(&self, expected: &Self) -> bool {
        if self == expected {
            return true;
        }
        eprintln!();
        eprintln!(
            "Textures: expected={}, actual={}",
            expected.textures.len(),
            self.textures.len()
        );
        for (i, (ta, te)) in self.textures.iter().zip(&expected.textures).enumerate() {
            if ta != te {
                eprintln!("expected[{}] = {:#?}", i, te);
                eprintln!("actual[{}] = {:#?}", i, ta);
                break;
            }
        }
        eprintln!();
        eprintln!(
            "Commands: expected={}, actual={}",
            expected.commands.len(),
            self.commands.len()
        );
        for (i, (ca, ce)) in self.commands.iter().zip(&expected.commands).enumerate() {
            if ca != ce {
                eprintln!("expected[{}] = {:#?}", i, ce);
                eprintln!("actual[{}] = {:#?}", i, ca);
                break;
            }
        }
        eprintln!();
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextureState {
    pub(crate) data: Option<TextureData>,
    pub(crate) sampler: Option<SamplerState>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TextureData {
    pub(crate) rgba8: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl fmt::Debug for TextureData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextureData")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SamplerState {
    pub(crate) linear_filter: bool,
    pub(crate) cms: u32,
    pub(crate) cmt: u32,
}

#[derive(Clone, PartialEq)]
pub(crate) struct DrawCommand {
    pub(crate) viewport: ScreenRectangle,
    pub(crate) scissor: ScreenRectangle,
    pub(crate) state: RenderState,
    /// Always has length 2. Uses Vec for the Diff implementation
    pub(crate) texture_index: [Option<usize>; 2],
    pub(crate) vertex_buffer: Vec<f32>,
    pub(crate) num_tris: usize,
}

impl fmt::Debug for DrawCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawCommand")
            .field("viewport", &self.viewport)
            .field("scissor", &self.scissor)
            .field("state", &self.state)
            .field("texture_index", &self.texture_index)
            .field("num_tris", &self.num_tris)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    pub(crate) cull_mode: CullMode,
}
