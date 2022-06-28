use wafel_memory::{DllGameMemory, DllSlot, MemoryError};

use crate::render_api::{
    decode_shader_id, render_display_list_with_backend, RenderBackend, ShaderId, ShaderInfo,
};

#[derive(Debug, Default)]
pub struct SM64RenderData {
    pub textures: Vec<Texture>,
    pub commands: Vec<Command>,
}

#[derive(Debug)]
pub struct Texture {
    pub data: Option<TextureData>,
    pub sampler: Option<SamplerState>,
}

#[derive(Debug)]
pub struct TextureData {
    pub rgba8: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerState {
    pub linear_filter: bool,
    pub cms: u32,
    pub cmt: u32,
}

#[derive(Debug)]
pub struct Command {
    pub viewport: Rectangle,
    pub scissor: Rectangle,
    pub state: RenderState,
    pub texture_index: [Option<usize>; 2],
    pub vertex_buffer: Vec<f32>,
    pub num_tris: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RenderState {
    pub shader_id: Option<u32>,
    pub depth_test: bool,
    pub depth_mask: bool,
    pub zmode_decal: bool,
    pub use_alpha: bool,
}

pub fn sm64_render_display_list(
    memory: &DllGameMemory,
    base_slot: &mut DllSlot,
    width: u32,
    height: u32,
) -> Result<SM64RenderData, MemoryError> {
    // We create a new backend every frame to ensure that gfx_pc isn't relying on rendering
    // state across frames (o/w frame rewind might break)
    let mut backend = SM64Backend::default();
    render_display_list_with_backend(memory, base_slot, &mut backend, width, height)?;
    Ok(backend.data)
}

#[derive(Debug, Default)]
struct SM64Backend {
    viewport: Rectangle,
    scissor: Rectangle,
    state: RenderState,
    tile: usize,
    texture_index: [Option<usize>; 2],
    data: SM64RenderData,
}

impl RenderBackend for SM64Backend {
    fn z_is_from_0_to_1(&self) -> bool {
        true
    }

    fn unload_shader(&mut self, _old_prg: ShaderId) {
        self.state.shader_id = None;
    }

    fn load_shader(&mut self, new_prg: ShaderId) {
        let shader_id = new_prg.0 as u32;
        self.state.shader_id = Some(shader_id);
    }

    fn create_and_load_new_shader(&mut self, shader_id: u32) -> ShaderId {
        self.state.shader_id = Some(shader_id);
        ShaderId(shader_id as usize)
    }

    fn lookup_shader(&self, shader_id: u32) -> Option<ShaderId> {
        Some(ShaderId(shader_id as usize))
    }

    fn shader_get_info(&self, prg: ShaderId) -> ShaderInfo {
        let shader_id = prg.0 as u32;
        let features = decode_shader_id(shader_id);
        ShaderInfo {
            num_inputs: features.num_inputs as u8,
            used_textures: features.used_textures,
        }
    }

    fn new_texture(&mut self) -> u32 {
        let id = self.data.textures.len() as u32;
        self.data.textures.push(Texture {
            data: None,
            sampler: None,
        });
        id
    }

    fn select_texture(&mut self, tile: i32, texture_id: u32) {
        assert!(
            (texture_id as usize) < self.data.textures.len(),
            "invalid texture id"
        );
        self.tile = tile as usize;
        self.texture_index[self.tile] = Some(texture_id as usize);
    }

    fn upload_texture(&mut self, rgba32_buf: &[u8], width: i32, height: i32) {
        assert!(4 * width * height == rgba32_buf.len() as i32);
        let texture_index = self.texture_index[self.tile].expect("no selected texture");
        self.data.textures[texture_index].data = Some(TextureData {
            rgba8: rgba32_buf.to_vec(),
            width: width as u32,
            height: height as u32,
        });
    }

    fn set_sampler_parameters(&mut self, tile: i32, linear_filter: bool, cms: u32, cmt: u32) {
        self.tile = tile as usize;
        let texture_index = self.texture_index[self.tile].expect("no selected texture");
        self.data.textures[texture_index].sampler = Some(SamplerState {
            linear_filter,
            cms,
            cmt,
        });
    }

    fn set_depth_test(&mut self, depth_test: bool) {
        self.state.depth_test = depth_test;
    }

    fn set_depth_mask(&mut self, z_upd: bool) {
        self.state.depth_mask = z_upd;
    }

    fn set_zmode_decal(&mut self, zmode_decal: bool) {
        self.state.zmode_decal = zmode_decal;
    }

    fn set_viewport(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.viewport = Rectangle {
            x,
            y,
            width,
            height,
        };
    }

    fn set_scissor(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.scissor = Rectangle {
            x,
            y,
            width,
            height,
        };
    }

    fn set_use_alpha(&mut self, use_alpha: bool) {
        self.state.use_alpha = use_alpha;
    }

    fn draw_triangles(&mut self, buf_vbo: &[f32], buf_vbo_num_tris: usize) {
        self.data.commands.push(Command {
            viewport: self.viewport,
            scissor: self.scissor,
            state: self.state,
            texture_index: self.texture_index,
            vertex_buffer: buf_vbo.to_vec(),
            num_tris: buf_vbo_num_tris,
        });
    }

    fn on_resize(&mut self) {}

    fn start_frame(&mut self) {}

    fn end_frame(&mut self) {}

    fn finish_render(&mut self) {}
}
