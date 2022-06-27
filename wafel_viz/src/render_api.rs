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
    fn unload_shader(&mut self, old_prg: ShaderId);
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
    fn draw_triangles(&mut self, buf_vbo: &[f32], buf_vbo_num_tris: usize);
    fn on_resize(&mut self);
    fn start_frame(&mut self);
    fn end_frame(&mut self);
    fn finish_render(&mut self);
}

pub fn update_and_render_with_backend(
    memory: &DllGameMemory,
    base_slot: &mut DllSlot,
    backend: &mut impl RenderBackend,
    width: u32,
    height: u32,
) -> Result<(), MemoryError> {
    unsafe {
        using_global_backend(backend, || -> Result<(), MemoryError> {
            {
                let set_render_api: unsafe extern "C" fn(*const RenderApi<ShaderProgram>) =
                    memory.symbol_pointer(base_slot, "sm64_set_render_api")?;
                set_render_api(&RENDER_API);
            }
            {
                let update_and_render: unsafe extern "C" fn(u32, u32) =
                    memory.symbol_pointer(base_slot, "sm64_update_and_render")?;
                update_and_render(width, height);
            }
            Ok(())
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CCFeatures {
    pub c: [[ShaderItem; 4]; 2],
    pub opt_alpha: bool,
    pub opt_fog: bool,
    pub opt_texture_edge: bool,
    pub opt_noise: bool,
    pub used_textures: [bool; 2],
    pub num_inputs: u32,
    pub do_single: [bool; 2],
    pub do_multiply: [bool; 2],
    pub do_mix: [bool; 2],
    pub color_alpha_same: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShaderItem {
    Zero,
    Input1,
    Input2,
    Input3,
    Input4,
    Texel0,
    Texel0A,
    Texel1,
}

impl Default for ShaderItem {
    fn default() -> Self {
        Self::Zero
    }
}

pub fn decode_shader_id(shader_id: u32) -> CCFeatures {
    // Copied from src/pc/gfx/gfx_cc.c (to avoid needing to call it with DllGameMemory)

    use ShaderItem::*;

    let mut cc_features = CCFeatures::default();

    for i in 0..4 {
        cc_features.c[0][i] = decode_item((shader_id >> (i * 3)) & 7);
        cc_features.c[1][i] = decode_item((shader_id >> (12 + i * 3)) & 7);
    }

    cc_features.opt_alpha = (shader_id & (1 << 24)) != 0;
    cc_features.opt_fog = (shader_id & (1 << 25)) != 0;
    cc_features.opt_texture_edge = (shader_id & (1 << 26)) != 0;
    cc_features.opt_noise = (shader_id & (1 << 27)) != 0;

    cc_features.used_textures[0] = false;
    cc_features.used_textures[1] = false;
    cc_features.num_inputs = 0;

    for i in 0..2 {
        for j in 0..4 {
            if cc_features.c[i][j] >= Input1 && cc_features.c[i][j] <= Input4 {
                let num_inputs0 = encode_item(cc_features.c[i][j]);
                if num_inputs0 > cc_features.num_inputs {
                    cc_features.num_inputs = num_inputs0;
                }
            }
            if cc_features.c[i][j] == Texel0 || cc_features.c[i][j] == Texel0A {
                cc_features.used_textures[0] = true;
            }
            if cc_features.c[i][j] == Texel1 {
                cc_features.used_textures[1] = true;
            }
        }
    }
    //
    cc_features.do_single[0] = cc_features.c[0][2] == Zero;
    cc_features.do_single[1] = cc_features.c[1][2] == Zero;
    cc_features.do_multiply[0] = cc_features.c[0][1] == Zero && cc_features.c[0][3] == Zero;
    cc_features.do_multiply[1] = cc_features.c[1][1] == Zero && cc_features.c[1][3] == Zero;
    cc_features.do_mix[0] = cc_features.c[0][1] == cc_features.c[0][3];
    cc_features.do_mix[1] = cc_features.c[1][1] == cc_features.c[1][3];
    cc_features.color_alpha_same = (shader_id & 0xfff) == ((shader_id >> 12) & 0xfff);

    cc_features
}

fn decode_item(v: u32) -> ShaderItem {
    match v {
        0 => ShaderItem::Zero,
        1 => ShaderItem::Input1,
        2 => ShaderItem::Input2,
        3 => ShaderItem::Input3,
        4 => ShaderItem::Input4,
        5 => ShaderItem::Texel0,
        6 => ShaderItem::Texel0A,
        7 => ShaderItem::Texel1,
        _ => unreachable!(),
    }
}

fn encode_item(item: ShaderItem) -> u32 {
    match item {
        ShaderItem::Zero => 0,
        ShaderItem::Input1 => 1,
        ShaderItem::Input2 => 2,
        ShaderItem::Input3 => 3,
        ShaderItem::Input4 => 4,
        ShaderItem::Texel0 => 5,
        ShaderItem::Texel0A => 6,
        ShaderItem::Texel1 => 7,
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
extern "C" fn unload_shader(old_prg: *const ShaderProgram) {
    use_backend(|b| {
        if let Some(shader) = id(old_prg) {
            b.unload_shader(shader)
        }
    })
}
extern "C" fn load_shader(new_prg: *const ShaderProgram) {
    let shader = id(new_prg).expect("null passed to load_shader");
    use_backend(|b| b.load_shader(shader))
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
    let shader = id(prg).expect("null passed to shader_get_info");
    use_backend(|b| {
        let info = b.shader_get_info(shader);
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
        b.draw_triangles(buf, buf_vbo_num_tris)
    })
}
extern "C" fn init() {}
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
