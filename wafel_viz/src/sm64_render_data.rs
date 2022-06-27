use std::mem;

use wafel_memory::{DllGameMemory, DllSlot, MemoryError};

use crate::render_api::{
    decode_shader_id, update_and_render_with_backend, CCFeatures, RenderBackend, ShaderId,
    ShaderInfo,
};

#[derive(Debug, Default)]
pub struct SM64RenderData {
    pub vertex_buffers: Vec<VertexBuffer>,
}

#[derive(Debug)]
pub struct VertexBuffer {
    pub buffer: Vec<f32>,
    pub num_tris: usize,
}

// #[derive(Debug)]
// pub struct RenderState {
//     shader_id: u32,
//     shader_features: CCFeatures,
// }

pub fn sm64_update_and_render(
    memory: &DllGameMemory,
    base_slot: &mut DllSlot,
    width: u32,
    height: u32,
) -> Result<SM64RenderData, MemoryError> {
    // We create a new backend every frame to ensure that gfx_pc isn't relying on rendering
    // state across frames (o/w frame rewind might break)
    let mut backend = SM64Backend::default();
    update_and_render_with_backend(memory, base_slot, &mut backend, width, height)?;
    Ok(backend.data)
}

#[derive(Debug, Default)]
struct SM64Backend {
    shader_id: Option<u32>,
    data: SM64RenderData,
}

#[derive(Debug)]
struct Shader {
    id: u32,
    features: CCFeatures,
}

impl RenderBackend for SM64Backend {
    fn z_is_from_0_to_1(&self) -> bool {
        true
    }

    fn unload_shader(&mut self, old_prg: ShaderId) {
        // eprintln!("unload_shader({:?})", old_prg);
        self.shader_id = None;
    }

    fn load_shader(&mut self, new_prg: ShaderId) {
        let shader_id = new_prg.0 as u32;
        eprintln!("load_shader({:#010X})", shader_id);
        self.shader_id = Some(shader_id);
    }

    fn create_and_load_new_shader(&mut self, shader_id: u32) -> ShaderId {
        eprintln!("create_and_load_new_shader({:#010X})", shader_id);
        self.shader_id = Some(shader_id);
        ShaderId(shader_id as usize)
    }

    fn lookup_shader(&self, shader_id: u32) -> Option<ShaderId> {
        // eprintln!("lookup_shader({:#010X})", shader_id);
        Some(ShaderId(shader_id as usize))
    }

    fn shader_get_info(&self, prg: ShaderId) -> ShaderInfo {
        // eprintln!("shader_get_info({:?})", prg);
        let shader_id = prg.0 as u32;
        let features = decode_shader_id(shader_id);
        let info = ShaderInfo {
            num_inputs: features.num_inputs as u8,
            used_textures: [false, false],
        };
        // eprintln!("  -> {:?}", info);
        info
    }

    fn new_texture(&mut self) -> u32 {
        // eprintln!("new_texture()");
        todo!()
    }

    fn select_texture(&mut self, tile: i32, texture_id: u32) {
        // eprintln!("select_texture({}, {})", tile, texture_id);
    }

    fn upload_texture(&mut self, rgba32_buf: &[u8], width: i32, height: i32) {
        // eprintln!("upload_texture({}, {})", width, height);
    }

    fn set_sampler_parameters(&mut self, sampler: i32, linear_filter: bool, cms: u32, cmt: u32) {
        // eprintln!(
        //     "set_sampler_parameters({}, {}, {}, {})",
        //     sampler, linear_filter, cms, cmt
        // );
    }

    fn set_depth_test(&mut self, depth_test: bool) {
        // eprintln!("set_depth_test({})", depth_test);
    }

    fn set_depth_mask(&mut self, z_upd: bool) {
        // eprintln!("set_depth_mask({})", z_upd);
    }

    fn set_zmode_decal(&mut self, zmode_decal: bool) {
        // eprintln!("set_zmode_decal({})", zmode_decal)
    }

    fn set_viewport(&mut self, x: i32, y: i32, width: i32, height: i32) {
        // eprintln!("set_viewport({}, {}, {}, {})", x, y, width, height);
    }

    fn set_scissor(&mut self, x: i32, y: i32, width: i32, height: i32) {
        // eprintln!("set_scissor({}, {}, {}, {})", x, y, width, height);
    }

    fn set_use_alpha(&mut self, use_alpha: bool) {
        // eprintln!("set_use_alpha({})", use_alpha);
    }

    fn draw_triangles(&mut self, buf_vbo: &[f32], buf_vbo_num_tris: usize) {
        // let stride = buf_vbo.len() / (3 * buf_vbo_num_tris);
        // if stride != 4 {
        //     eprintln!("stride = {}", stride);
        // }
        eprintln!(
            "  draw_triangles({}, {}, stride={})",
            buf_vbo.len(),
            buf_vbo_num_tris,
            buf_vbo.len() / (3 * buf_vbo_num_tris)
        );
        self.data.vertex_buffers.push(VertexBuffer {
            buffer: buf_vbo.to_vec(),
            num_tris: buf_vbo_num_tris,
        });
    }

    fn on_resize(&mut self) {
        // eprintln!("on_resize()");
    }

    fn start_frame(&mut self) {
        // eprintln!("start_frame()");
    }

    fn end_frame(&mut self) {
        // eprintln!("end_frame()");
    }

    fn finish_render(&mut self) {
        // eprintln!("finish_render()");
    }
}
