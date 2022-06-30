use wafel_memory::{DllGameMemory, DllSlot, MemoryError};

use crate::{
    n64_render_data::{
        DrawCommand, N64RenderData, RenderState, SamplerState, ScreenRectangle, TextureData,
        TextureState,
    },
    render_api::{
        decode_shader_id, process_display_list_with_backend, RenderBackend, ShaderId, ShaderInfo,
    },
};

pub fn process_display_list(
    memory: &DllGameMemory,
    base_slot: &mut DllSlot,
    width: u32,
    height: u32,
) -> Result<N64RenderData, MemoryError> {
    // We create a new backend every frame to ensure that gfx_pc isn't relying on rendering
    // state across frames (o/w frame rewind might break)
    let mut backend = N64RenderBackend::default();
    process_display_list_with_backend(memory, base_slot, &mut backend, width, height)?;
    Ok(backend.finish())
}

#[derive(Debug, Default)]
pub struct N64RenderBackend {
    viewport: ScreenRectangle,
    scissor: ScreenRectangle,
    state: RenderState,
    tile: usize,
    texture_index: [Option<usize>; 2],
    data: N64RenderData,
}

impl N64RenderBackend {
    pub fn finish(self) -> N64RenderData {
        self.data
    }
}

impl RenderBackend for N64RenderBackend {
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
        self.data.textures.push(TextureState {
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
        self.viewport = ScreenRectangle {
            x,
            y,
            width,
            height,
        };
    }

    fn set_scissor(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.scissor = ScreenRectangle {
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
        self.data.commands.push(DrawCommand {
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
