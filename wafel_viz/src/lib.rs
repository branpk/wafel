#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

use std::error::Error;

use render_api::{init_render_api, update_and_render, RenderBackend, ShaderId, ShaderInfo};
use wafel_memory::DllGameMemory;

mod render_api;

#[derive(Debug)]
struct Backend {}

impl RenderBackend for Backend {
    fn z_is_from_0_to_1(&self) -> bool {
        eprintln!("z_is_from_0_to_1()");
        true
    }

    fn unload_shader(&mut self, old_prod: ShaderId) {
        eprintln!("unload_shader({:?})", old_prod);
    }

    fn load_shader(&mut self, new_prg: ShaderId) {
        eprintln!("load_shader({:?})", new_prg);
    }

    fn create_and_load_new_shader(&mut self, shader_id: u32) -> ShaderId {
        eprintln!("create_and_load_new_shader({:#08X})", shader_id);
        todo!()
    }

    fn lookup_shader(&self, shader_id: u32) -> Option<ShaderId> {
        eprintln!("lookup_shader({:#08X})", shader_id);
        let shader: ShaderId = ShaderId(12);
        eprintln!("  -> {:?}", shader);
        Some(shader)
    }

    fn shader_get_info(&self, prg: ShaderId) -> ShaderInfo {
        eprintln!("shader_get_info({:?})", prg);
        let info = ShaderInfo {
            num_inputs: 0,
            used_textures: [false, false],
        };
        eprintln!("  -> {:?}", info);
        info
    }

    fn new_texture(&mut self) -> u32 {
        eprintln!("new_texture()");
        todo!()
    }

    fn select_texture(&mut self, tile: i32, texture_id: u32) {
        eprintln!("select_texture({}, {})", tile, texture_id);
    }

    fn upload_texture(&mut self, rgba32_buf: &[u8], width: i32, height: i32) {
        eprintln!("upload_texture({}, {})", width, height);
    }

    fn set_sampler_parameters(&mut self, sampler: i32, linear_filter: bool, cms: u32, cmt: u32) {
        eprintln!(
            "set_sampler_parameters({}, {}, {}, {})",
            sampler, linear_filter, cms, cmt
        );
    }

    fn set_depth_test(&mut self, depth_test: bool) {
        eprintln!("set_depth_test({})", depth_test);
    }

    fn set_depth_mask(&mut self, z_upd: bool) {
        eprintln!("set_depth_mask({})", z_upd);
    }

    fn set_zmode_decal(&mut self, zmode_decal: bool) {
        eprintln!("set_zmode_decal({})", zmode_decal)
    }

    fn set_viewport(&mut self, x: i32, y: i32, width: i32, height: i32) {
        eprintln!("set_viewport({}, {}, {}, {})", x, y, width, height);
    }

    fn set_scissor(&mut self, x: i32, y: i32, width: i32, height: i32) {
        eprintln!("set_scissor({}, {}, {}, {})", x, y, width, height);
    }

    fn set_use_alpha(&mut self, use_alpha: bool) {
        eprintln!("set_use_alpha({})", use_alpha);
    }

    fn draw_triangles(&mut self, buf_vbo: &[f32], buf_vbo_len: usize, buf_vbo_num_tris: usize) {
        eprintln!("draw_triangles({}, {})", buf_vbo_len, buf_vbo_num_tris);
    }

    fn init(&mut self) {
        eprintln!("init()");
    }

    fn on_resize(&mut self) {
        eprintln!("on_resize()");
    }

    fn start_frame(&mut self) {
        eprintln!("start_frame()");
    }

    fn end_frame(&mut self) {
        eprintln!("end_frame()");
    }

    fn finish_render(&mut self) {
        eprintln!("finish_render()");
    }
}

pub fn test() -> Result<(), Box<dyn Error>> {
    let (memory, mut base_slot) = unsafe {
        DllGameMemory::load(
            "../libsm64-build/build/us_lib/sm64_us.dll",
            "sm64_init",
            "sm64_update",
        )?
    };
    let mut backend = Backend {};

    init_render_api(&memory, &mut base_slot, &mut backend);

    update_and_render(&memory, &mut base_slot, &mut backend, 640, 480)?;

    Ok(())
}
