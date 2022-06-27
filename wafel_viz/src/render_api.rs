use std::{ptr, slice};

use wafel_memory::{DllGameMemory, DllSlot, MemoryError};

use self::global_backend::{use_backend, using_global_backend};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShaderId(pub usize);

#[derive(Debug, Clone, Copy)]
pub struct ShaderInfo {
    pub num_inputs: u8,
    pub used_textures: [bool; 2],
}

pub trait RenderBackend {
    fn z_is_from_0_to_1(&self) -> bool;
    fn unload_shader(&mut self, old_prod: ShaderId);
    fn load_shader(&mut self, new_prg: ShaderId);
    fn create_and_load_new_shader(&mut self, shader_id: u32) -> ShaderId;
    fn lookup_shader(&self, shader_id: u32) -> Option<ShaderId>;
    fn shader_get_info(&self, prg: ShaderId) -> ShaderInfo;
    fn new_texture(&mut self) -> u32;
    fn select_texture(&mut self, tile: i32, texture_id: u32);
    fn upload_texture(&mut self, rgba32_buf: &[u8], width: i32, height: i32);
    fn set_sampler_parameters(&mut self, sampler: i32, linear_filter: bool, cms: u32, cmt: u32);
    fn set_depth_test(&mut self, depth_test: bool);
    fn set_depth_mask(&mut self, z_upd: bool);
    fn set_zmode_decal(&mut self, zmode_decal: bool);
    fn set_viewport(&mut self, x: i32, y: i32, width: i32, height: i32);
    fn set_scissor(&mut self, x: i32, y: i32, width: i32, height: i32);
    fn set_use_alpha(&mut self, use_alpha: bool);
    fn draw_triangles(&mut self, buf_vbo: &[f32], buf_vbo_len: usize, buf_vbo_num_tris: usize);
    fn init(&mut self);
    fn on_resize(&mut self);
    fn start_frame(&mut self);
    fn end_frame(&mut self);
    fn finish_render(&mut self);
}

pub fn init_render_api(
    memory: &DllGameMemory,
    base_slot: &mut DllSlot,
    backend: &mut impl RenderBackend,
) -> Result<(), MemoryError> {
    unsafe {
        using_global_backend(backend, || -> Result<(), MemoryError> {
            {
                let init_render_api: unsafe extern "C" fn(*const RenderApi<ShaderProgram>) =
                    memory.symbol_pointer(base_slot, "init_render_api")?;
                init_render_api(&RENDER_API);
            }
            Ok(())
        })
    }
}

pub fn update_and_render(
    memory: &DllGameMemory,
    base_slot: &mut DllSlot,
    backend: &mut impl RenderBackend,
    width: u32,
    height: u32,
) -> Result<(), MemoryError> {
    unsafe {
        using_global_backend(backend, || -> Result<(), MemoryError> {
            {
                let update_and_render: unsafe extern "C" fn(u32, u32) =
                    memory.symbol_pointer(base_slot, "sm64_update_and_render")?;
                update_and_render(width, height);
            }
            Ok(())
        })
    }
}

#[repr(C)]
#[derive(Debug)]
struct RenderApi<S> {
    z_is_from_0_to_1: extern "C" fn() -> bool,
    unload_shader: extern "C" fn(old_prod: *const S),
    load_shader: extern "C" fn(new_prg: *const S),
    create_and_load_new_shader: extern "C" fn(shader_id: u32) -> *const S,
    lookup_shader: extern "C" fn(shader_id: u32) -> *const S,
    shader_get_info: extern "C" fn(prg: *const S, num_inputs: *mut u8, used_textures: *mut bool),
    new_texture: extern "C" fn() -> u32,
    select_texture: extern "C" fn(tile: i32, texture_id: u32),
    upload_texture: extern "C" fn(rgba32_buf: *const u8, width: i32, height: i32),
    set_sampler_parameters: extern "C" fn(sampler: i32, linear_filter: bool, cms: u32, cmt: u32),
    set_depth_test: extern "C" fn(depth_test: bool),
    set_depth_mask: extern "C" fn(z_upd: bool),
    set_zmode_decal: extern "C" fn(zmode_decal: bool),
    set_viewport: extern "C" fn(x: i32, y: i32, width: i32, height: i32),
    set_scissor: extern "C" fn(x: i32, y: i32, width: i32, height: i32),
    set_use_alpha: extern "C" fn(use_alpha: bool),
    draw_triangles: extern "C" fn(buf_vbo: *const f32, buf_vbo_len: usize, buf_vbo_num_tris: usize),
    init: extern "C" fn(),
    on_resize: extern "C" fn(),
    start_frame: extern "C" fn(),
    end_frame: extern "C" fn(),
    finish_render: extern "C" fn(),
}

#[repr(C)]
struct ShaderProgram;

impl From<ShaderId> for *const ShaderProgram {
    fn from(id: ShaderId) -> Self {
        (id.0 + 1) as *const ShaderProgram
    }
}

fn ptr(shader: Option<ShaderId>) -> *const ShaderProgram {
    match shader {
        Some(id) => (id.0 + 1) as *const ShaderProgram,
        None => ptr::null(),
    }
}

fn id(program: *const ShaderProgram) -> Option<ShaderId> {
    let p = program as usize;
    if p == 0 {
        None
    } else {
        Some(ShaderId(p - 1))
    }
}

static RENDER_API: RenderApi<ShaderProgram> = RenderApi {
    z_is_from_0_to_1,
    unload_shader,
    load_shader,
    create_and_load_new_shader,
    lookup_shader,
    shader_get_info,
    new_texture,
    select_texture,
    upload_texture,
    set_sampler_parameters,
    set_depth_test,
    set_depth_mask,
    set_zmode_decal,
    set_viewport,
    set_scissor,
    set_use_alpha,
    draw_triangles,
    init,
    on_resize,
    start_frame,
    end_frame,
    finish_render,
};

extern "C" fn z_is_from_0_to_1() -> bool {
    use_backend(|b| b.z_is_from_0_to_1())
}
extern "C" fn unload_shader(old_prod: *const ShaderProgram) {
    use_backend(|b| {
        if let Some(shader) = id(old_prod) {
            b.unload_shader(shader)
        }
    })
}
extern "C" fn load_shader(new_prg: *const ShaderProgram) {
    use_backend(|b| {
        if let Some(shader) = id(new_prg) {
            b.load_shader(shader)
        }
    })
}
extern "C" fn create_and_load_new_shader(shader_id: u32) -> *const ShaderProgram {
    use_backend(|b| b.create_and_load_new_shader(shader_id).into())
}
extern "C" fn lookup_shader(shader_id: u32) -> *const ShaderProgram {
    use_backend(|b| ptr(b.lookup_shader(shader_id)))
}
extern "C" fn shader_get_info(
    prg: *const ShaderProgram,
    num_inputs: *mut u8,
    used_textures: *mut bool,
) {
    use_backend(|b| {
        let info = match id(prg) {
            Some(shader) => b.shader_get_info(shader),
            None => ShaderInfo {
                num_inputs: 0,
                used_textures: [false, false],
            },
        };
        unsafe {
            *num_inputs = info.num_inputs;
            *used_textures.offset(0) = info.used_textures[0];
            *used_textures.offset(1) = info.used_textures[1];
        }
    })
}
extern "C" fn new_texture() -> u32 {
    use_backend(|b| b.new_texture())
}
extern "C" fn select_texture(tile: i32, texture_id: u32) {
    use_backend(|b| b.select_texture(tile, texture_id))
}
extern "C" fn upload_texture(rgba32_buf: *const u8, width: i32, height: i32) {
    use_backend(|b| unsafe {
        let buf = slice::from_raw_parts(rgba32_buf, (4 * width * height) as usize);
        b.upload_texture(buf, width, height)
    })
}
extern "C" fn set_sampler_parameters(sampler: i32, linear_filter: bool, cms: u32, cmt: u32) {
    use_backend(|b| b.set_sampler_parameters(sampler, linear_filter, cms, cmt))
}
extern "C" fn set_depth_test(depth_test: bool) {
    use_backend(|b| b.set_depth_test(depth_test))
}
extern "C" fn set_depth_mask(z_upd: bool) {
    use_backend(|b| b.set_depth_mask(z_upd))
}
extern "C" fn set_zmode_decal(zmode_decal: bool) {
    use_backend(|b| b.set_zmode_decal(zmode_decal))
}
extern "C" fn set_viewport(x: i32, y: i32, width: i32, height: i32) {
    use_backend(|b| b.set_viewport(x, y, width, height))
}
extern "C" fn set_scissor(x: i32, y: i32, width: i32, height: i32) {
    use_backend(|b| b.set_scissor(x, y, width, height))
}
extern "C" fn set_use_alpha(use_alpha: bool) {
    use_backend(|b| b.set_use_alpha(use_alpha))
}
extern "C" fn draw_triangles(buf_vbo: *const f32, buf_vbo_len: usize, buf_vbo_num_tris: usize) {
    use_backend(|b| unsafe {
        let buf = slice::from_raw_parts(buf_vbo, buf_vbo_len);
        b.draw_triangles(buf, buf_vbo_len, buf_vbo_num_tris)
    })
}
extern "C" fn init() {
    use_backend(|b| b.init())
}
extern "C" fn on_resize() {
    use_backend(|b| b.on_resize())
}
extern "C" fn start_frame() {
    use_backend(|b| b.start_frame())
}
extern "C" fn end_frame() {
    use_backend(|b| b.end_frame())
}
extern "C" fn finish_render() {
    use_backend(|b| b.finish_render())
}

/// Sketchy implementation of allowing the C rendering API to access RenderBackend state.
mod global_backend {
    use std::{mem, sync::Mutex};

    use once_cell::sync::OnceCell;

    use super::RenderBackend;

    struct RenderBackendPointer(*mut dyn RenderBackend);

    unsafe impl Send for RenderBackendPointer {}
    unsafe impl Sync for RenderBackendPointer {}

    fn backend_pointer_mutex() -> &'static Mutex<Option<RenderBackendPointer>> {
        static BACKEND_POINTER: OnceCell<Mutex<Option<RenderBackendPointer>>> = OnceCell::new();
        BACKEND_POINTER.get_or_init(|| Mutex::new(None))
    }

    pub(super) fn using_global_backend<T>(
        backend: &mut dyn RenderBackend,
        action: impl FnOnce() -> T,
    ) -> T {
        {
            let mut global_pointer = backend_pointer_mutex().lock().unwrap();

            // Not totally safe, but usually the global pointer will be reset at the end of this
            // function, so backend will outlive it
            unsafe {
                let pointer: *mut dyn RenderBackend =
                    mem::transmute::<_, &'static mut dyn RenderBackend>(backend);
                *global_pointer = Some(RenderBackendPointer(pointer));
            }
        }

        let result = action();

        *backend_pointer_mutex().lock().unwrap() = None;
        result
    }

    #[track_caller]
    pub(super) fn use_backend<T>(action: impl FnOnce(&mut dyn RenderBackend) -> T) -> T {
        let global_pointer = backend_pointer_mutex().lock().unwrap();
        assert!(global_pointer.is_some(), "no render backend set");
        unsafe {
            let pointer: *mut dyn RenderBackend = global_pointer.as_ref().unwrap().0;
            let backend: &mut dyn RenderBackend = &mut *pointer;
            action(backend)
        }
    }
}
