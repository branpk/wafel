use std::{f32::consts::PI, sync::Arc};

use bytemuck::cast_slice;
use fast3d::{
    decode::{
        decode_f3d_display_list, F3DCommand, F3DCommandIter, GeometryModes, MatrixMode, MatrixOp,
        RawF3DCommand, SPCommand,
    },
    interpret::{interpret_f3d_display_list, F3DMemory, F3DRenderData},
    util::Matrixf,
};
use wafel_api::{Address, Error, IntType};
use wafel_data_path::GlobalDataPath;
use wafel_data_type::DataType;
use wafel_memory::{MemoryError, MemoryRead};

#[derive(Debug, Clone, PartialEq)]
pub struct SM64RenderConfig {
    pub screen_size: (u32, u32),
    pub camera: Camera,
}

impl Default for SM64RenderConfig {
    fn default() -> Self {
        Self {
            screen_size: (320, 240),
            camera: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Camera {
    InGame,
    LookAt {
        pos: [f32; 3],
        focus: [f32; 3],
        roll: f32,
    },
}

impl Default for Camera {
    fn default() -> Self {
        Self::InGame
    }
}

pub fn render_sm64_with_config(
    memory: &impl MemoryRead,
    mut get_path: impl FnMut(&str) -> Result<Arc<GlobalDataPath>, Error>,
    config: &SM64RenderConfig,
) -> Result<F3DRenderData, Error> {
    let pool_array_type = get_path("gGfxPools")?.concrete_type();
    let pool_length = if let DataType::Array {
        length: Some(length),
        ..
    } = pool_array_type.as_ref()
    {
        *length
    } else {
        2
    };

    let global_timer = get_path("gGlobalTimer")?.read(memory)?.as_int();
    if global_timer == 1 {
        // libsm64 doesn't render in init
        return Ok(F3DRenderData::default());
    }

    let dl_buffer_index = (global_timer as usize + 1) % pool_length;
    let dl_addr = get_path(&format!("gGfxPools[{}].buffer", dl_buffer_index))?
        .address(memory)?
        .unwrap();

    let view_transform = match config.camera {
        Camera::InGame => None,
        Camera::LookAt { pos, focus, roll } => {
            let lakitu_pos = get_path("gLakituState.pos")?.read(memory)?.try_as_f32_3()?;
            let lakitu_focus = get_path("gLakituState.focus")?
                .read(memory)?
                .try_as_f32_3()?;
            let lakitu_roll = get_path("gLakituState.roll")?.read(memory)?.try_as_int()?;
            let lakitu_roll = lakitu_roll as f32 * PI / 0x8000 as f32;

            if pos != focus && lakitu_pos != lakitu_focus {
                let lakitu_view_mtx = Matrixf::look_at(lakitu_pos, lakitu_focus, lakitu_roll);
                let new_view_mtx = Matrixf::look_at(pos, focus, roll);
                Some(&new_view_mtx * &lakitu_view_mtx.invert_isometry())
            } else {
                None
            }
        }
    };

    let f3d_memory = F3DMemoryImpl {
        memory,
        root_addr: dl_addr,
        view_transform,
    };
    let render_data = interpret_f3d_display_list(&f3d_memory, config.screen_size, true)?;

    Ok(render_data)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Pointer {
    Address(Address),
    ViewMatrix(Address),
}

impl Pointer {
    fn exact_address(self) -> Address {
        if let Self::Address(addr) = self {
            addr
        } else {
            panic!("not an exact address: {:?}", self)
        }
    }

    fn address(self) -> Address {
        match self {
            Pointer::Address(addr) => addr,
            Pointer::ViewMatrix(addr) => addr,
        }
    }
}

#[derive(Debug)]
struct F3DMemoryImpl<'m, M> {
    memory: &'m M,
    root_addr: Address,
    view_transform: Option<Matrixf>,
}

impl<'m, M: MemoryRead> F3DMemoryImpl<'m, M> {
    fn read_dl_impl(&self, addr: Address, is_root: bool) -> <Self as F3DMemory>::DlIter {
        let raw = RawDlIter {
            memory: self.memory,
            addr,
        };
        let decoded = decode_f3d_display_list(raw);
        DlTransformer::new(decoded, is_root)
    }
}

impl<'m, M: MemoryRead> F3DMemory for F3DMemoryImpl<'m, M> {
    type Ptr = Pointer;
    type Error = MemoryError;
    type DlIter = DlTransformer<F3DCommandIter<RawDlIter<'m, M>>>;

    fn root_dl(&self) -> Result<Self::DlIter, Self::Error> {
        Ok(self.read_dl_impl(self.root_addr, true))
    }

    fn read_dl(&self, ptr: Self::Ptr) -> Result<Self::DlIter, Self::Error> {
        Ok(self.read_dl_impl(ptr.exact_address(), false))
    }

    // TODO: Optimize buffer reads?

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr.exact_address() + offset;
        for i in 0..dst.len() {
            dst[i] = self.memory.read_int(addr + i, IntType::U8)? as u8;
        }
        Ok(())
    }

    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr.exact_address() + offset;
        for i in 0..dst.len() {
            dst[i] = self.memory.read_int(addr + 2 * i, IntType::U16)? as u16;
        }
        Ok(())
    }

    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr.address() + offset;
        for i in 0..dst.len() {
            dst[i] = self.memory.read_int(addr + 4 * i, IntType::U32)? as u32;
        }

        if matches!(ptr, Pointer::ViewMatrix(_)) {
            if let Some(view_transform) = self.view_transform.as_ref() {
                let mtx = Matrixf::from_fixed(cast_slice(dst));
                let mtx_new = view_transform * &mtx;
                dst.copy_from_slice(cast_slice(mtx_new.to_fixed().as_slice()));
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct DlTransformer<I> {
    inner: I,
    is_root: bool,
    z_buffer: bool,
}

impl<I> DlTransformer<I> {
    fn new(inner: I, is_root: bool) -> Self {
        Self {
            inner,
            is_root,
            z_buffer: false,
        }
    }

    fn transform(&mut self, mut cmd: F3DCommand<Pointer>) -> F3DCommand<Pointer> {
        if self.is_root {
            if let F3DCommand::Rsp(cmd) = &mut cmd {
                match cmd {
                    SPCommand::Matrix {
                        matrix, mode, op, ..
                    } => {
                        if *mode == MatrixMode::ModelView && *op == MatrixOp::Load {
                            *matrix = Pointer::ViewMatrix(matrix.exact_address());
                        }
                    }
                    SPCommand::SetGeometryMode(mode) => {
                        if mode.contains(GeometryModes::ZBUFFER) {
                            self.z_buffer = true;
                        }
                    }
                    SPCommand::ClearGeometryMode(mode) => {
                        if mode.contains(GeometryModes::ZBUFFER) {
                            self.z_buffer = false;
                        }
                    }
                    _ => {}
                }
            }
        }
        cmd
    }
}

impl<I> Iterator for DlTransformer<I>
where
    I: Iterator<Item = Result<F3DCommand<Pointer>, MemoryError>>,
{
    type Item = Result<F3DCommand<Pointer>, MemoryError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|result| result.map(|cmd| self.transform(cmd)))
    }
}

#[derive(Debug)]
struct RawDlIter<'m, M> {
    memory: &'m M,
    addr: Address,
}

impl<'m, M: MemoryRead> RawDlIter<'m, M> {
    fn next_impl(&mut self) -> Result<RawF3DCommand<Pointer>, MemoryError> {
        let w_type = self.memory.pointer_int_type();
        let w_size = w_type.size();

        let w0 = self.memory.read_int(self.addr, w_type)? as u32;
        self.addr += w_size;

        let w1 = self.memory.read_int(self.addr, w_type)? as u32;
        let w1_ptr = Pointer::Address(self.memory.read_address(self.addr)?);
        self.addr += w_size;

        Ok(RawF3DCommand { w0, w1, w1_ptr })
    }
}

impl<'m, M: MemoryRead> Iterator for RawDlIter<'m, M> {
    type Item = Result<RawF3DCommand<Pointer>, MemoryError>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_impl())
    }
}
