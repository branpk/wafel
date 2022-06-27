use wafel_memory::{DllGameMemory, DllSlot, MemoryError};

use crate::render_api::{update_and_render_with_backend, RenderBackend, ShaderId, ShaderInfo};

#[derive(Debug, Default)]
pub struct SM64RenderData {
    pub vertex_buffers: Vec<VertexBuffer>,
}

#[derive(Debug)]
pub struct VertexBuffer {
    pub buffer: Vec<f32>,
    pub num_tris: usize,
}

pub fn sm64_update_and_render(
    memory: &DllGameMemory,
    base_slot: &mut DllSlot,
    width: u32,
    height: u32,
) -> Result<SM64RenderData, MemoryError> {
    let mut backend = SM64Backend::default();
    update_and_render_with_backend(memory, base_slot, &mut backend, width, height)?;
    Ok(backend.data)
}

#[derive(Debug, Default)]
struct SM64Backend {
    data: SM64RenderData,
}

impl RenderBackend for SM64Backend {
    fn z_is_from_0_to_1(&self) -> bool {
        true
    }

    fn unload_shader(&mut self, old_prod: ShaderId) {
        // eprintln!("unload_shader({:?})", old_prod);
    }

    fn load_shader(&mut self, new_prg: ShaderId) {
        // eprintln!("load_shader({:?})", new_prg);
    }

    fn create_and_load_new_shader(&mut self, shader_id: u32) -> ShaderId {
        // eprintln!("create_and_load_new_shader({:#08X})", shader_id);
        todo!()
    }

    fn lookup_shader(&self, shader_id: u32) -> Option<ShaderId> {
        // eprintln!("lookup_shader({:#08X})", shader_id);
        let shader: ShaderId = ShaderId(12);
        // eprintln!("  -> {:?}", shader);
        Some(shader)
    }

    fn shader_get_info(&self, prg: ShaderId) -> ShaderInfo {
        // eprintln!("shader_get_info({:?})", prg);
        let info = ShaderInfo {
            num_inputs: 0,
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
        // eprintln!("draw_triangles({}, {})", buf_vbo_len, buf_vbo_num_tris);
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
