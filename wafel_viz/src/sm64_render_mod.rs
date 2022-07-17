use std::{f32::consts::PI, num::Wrapping, sync::Arc, vec};

use bytemuck::cast_slice;
use fast3d::{
    cmd::{F3DCommand, GeometryModes, MatrixMode, MatrixOp},
    decode::{decode_f3d_display_list, F3DCommandIter, RawF3DCommand},
    interpret::{interpret_f3d_display_list, F3DMemory, F3DRenderData},
    util::{Angle, Matrixf},
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
        roll: Angle,
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
    if let Some(dl_addr) = get_dl_addr(memory, &mut get_path)? {
        let view_transform = match config.camera {
            Camera::InGame => None,
            Camera::LookAt { pos, focus, roll } => {
                let lakitu_pos = get_path("gLakituState.pos")?.read(memory)?.try_as_f32_3()?;
                let lakitu_focus = get_path("gLakituState.focus")?
                    .read(memory)?
                    .try_as_f32_3()?;
                let lakitu_roll =
                    Wrapping(get_path("gLakituState.roll")?.read(memory)?.try_as_int()? as i16);

                if pos != focus && lakitu_pos != lakitu_focus {
                    let lakitu_view_mtx = Matrixf::look_at(lakitu_pos, lakitu_focus, lakitu_roll);
                    let new_view_mtx = Matrixf::look_at(pos, focus, roll);
                    Some(&new_view_mtx * &lakitu_view_mtx.invert_isometry())
                } else {
                    None
                }
            }
        };

        let mut f3d_memory = F3DMemoryImpl::new(memory, dl_addr.into());
        f3d_memory.view_transform = view_transform;
        let render_data = interpret_f3d_display_list(&f3d_memory, config.screen_size, true)?;

        Ok(render_data)
    } else {
        Ok(F3DRenderData::default())
    }
}

pub fn get_dl_addr(
    memory: &impl MemoryRead,
    mut get_path: impl FnMut(&str) -> Result<Arc<GlobalDataPath>, Error>,
) -> Result<Option<Address>, Error> {
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
        return Ok(None);
    }
    let dl_buffer_index = (global_timer as usize + 1) % pool_length;
    let dl_addr = get_path(&format!("gGfxPools[{}].buffer", dl_buffer_index))?
        .address(memory)?
        .unwrap();
    Ok(Some(dl_addr))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pointer {
    Address(Address),
    ViewMatrix(SimplePointer),
    BufferOffset(usize),
}

impl Pointer {
    #[track_caller]
    pub fn exact_address(self) -> Address {
        if let Self::Address(addr) = self {
            addr
        } else {
            panic!("not an exact address: {:?}", self)
        }
    }

    #[track_caller]
    pub fn address(self) -> Address {
        match self {
            Pointer::Address(addr) => addr,
            Pointer::ViewMatrix(ptr) => ptr.address(),
            _ => panic!("no address: {:?}", self),
        }
    }

    #[track_caller]
    pub fn simple(self) -> SimplePointer {
        match self {
            Pointer::Address(addr) => SimplePointer::Address(addr),
            Pointer::BufferOffset(offset) => SimplePointer::BufferOffset(offset),
            _ => panic!("not a simple pointer: {:?}", self),
        }
    }
}

impl From<Address> for Pointer {
    fn from(addr: Address) -> Self {
        Self::Address(addr)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimplePointer {
    Address(Address),
    BufferOffset(usize),
}

impl SimplePointer {
    #[track_caller]
    pub fn address(self) -> Address {
        match self {
            Self::Address(addr) => addr,
            _ => panic!("no address: {:?}", self),
        }
    }
}

#[derive(Debug)]
pub struct F3DMemoryImpl<'m, M> {
    memory: &'m M,
    root_ptr: Pointer,
    view_transform: Option<Matrixf>,
    dl_buffer: Vec<Vec<F3DCommand<Pointer>>>,
    u32_buffer: Vec<u32>,
}

impl<'m, M: MemoryRead> F3DMemoryImpl<'m, M> {
    pub fn new(memory: &'m M, root_ptr: Pointer) -> Self {
        Self {
            memory,
            root_ptr,
            view_transform: None,
            dl_buffer: Vec::new(),
            u32_buffer: Vec::new(),
        }
    }

    pub fn set_view_transform(&mut self, transform: Option<Matrixf>) {
        self.view_transform = transform;
    }

    pub fn set_dl_buffer(&mut self, buffer: Vec<Vec<F3DCommand<Pointer>>>) {
        self.dl_buffer = buffer;
    }

    pub fn set_u32_buffer(&mut self, buffer: Vec<u32>) {
        self.u32_buffer = buffer;
    }

    fn read_dl_impl(&self, ptr: Pointer, is_root: bool) -> <Self as F3DMemory>::DlIter {
        let cmd_iter = match ptr {
            Pointer::Address(addr) => {
                let raw = RawDlIter {
                    memory: self.memory,
                    addr,
                };
                DlIter::FromRaw(decode_f3d_display_list(raw))
            }
            Pointer::BufferOffset(offset) => {
                DlIter::FromVec(self.dl_buffer[offset].clone().into_iter())
            }
            _ => unimplemented!("{:?}", ptr),
        };
        DlTransformer::new(cmd_iter, is_root)
    }

    fn read_u32_simple(
        &self,
        dst: &mut [u32],
        ptr: SimplePointer,
        offset: usize,
    ) -> Result<(), MemoryError> {
        match ptr {
            SimplePointer::Address(addr) => {
                let addr = addr + offset;
                for i in 0..dst.len() {
                    dst[i] = self.memory.read_int(addr + 4 * i, IntType::U32)? as u32;
                }
            }
            SimplePointer::BufferOffset(offset) => {
                dst.copy_from_slice(&self.u32_buffer[offset..offset + dst.len()]);
            }
        }
        Ok(())
    }
}

impl<'m, M: MemoryRead> F3DMemory for F3DMemoryImpl<'m, M> {
    type Ptr = Pointer;
    type Error = MemoryError;
    type DlIter = DlTransformer<DlIter<'m, M>>;

    fn root_dl(&self) -> Result<Self::DlIter, Self::Error> {
        Ok(self.read_dl_impl(self.root_ptr, true))
    }

    fn read_dl(&self, ptr: Self::Ptr) -> Result<Self::DlIter, Self::Error> {
        Ok(self.read_dl_impl(ptr, false))
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
        if let Pointer::ViewMatrix(ptr) = ptr {
            self.read_u32_simple(dst, ptr, offset)?;
            if let Some(view_transform) = self.view_transform.as_ref() {
                let mtx = Matrixf::from_fixed(cast_slice(dst));
                let mtx_new = view_transform * &mtx;
                dst.copy_from_slice(cast_slice(mtx_new.to_fixed().as_slice()));
            }
        } else {
            self.read_u32_simple(dst, ptr.simple(), offset)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct DlTransformer<I> {
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
            match &mut cmd {
                F3DCommand::SPMatrix {
                    matrix, mode, op, ..
                } => {
                    // TODO: Check z buffer
                    if *mode == MatrixMode::ModelView && *op == MatrixOp::Load {
                        *matrix = Pointer::ViewMatrix(matrix.simple());
                    }
                }
                F3DCommand::SPSetGeometryMode(mode) => {
                    if mode.contains(GeometryModes::ZBUFFER) {
                        self.z_buffer = true;
                    }
                }
                F3DCommand::SPClearGeometryMode(mode) => {
                    if mode.contains(GeometryModes::ZBUFFER) {
                        self.z_buffer = false;
                    }
                }
                _ => {}
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
pub enum DlIter<'m, M> {
    FromVec(vec::IntoIter<F3DCommand<Pointer>>),
    FromRaw(F3DCommandIter<RawDlIter<'m, M>>),
}

impl<'m, M: MemoryRead> Iterator for DlIter<'m, M> {
    type Item = Result<F3DCommand<Pointer>, MemoryError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DlIter::FromVec(iter) => iter.next().map(Ok),
            DlIter::FromRaw(iter) => iter.next(),
        }
    }
}

#[derive(Debug)]
pub struct RawDlIter<'m, M> {
    pub memory: &'m M,
    pub addr: Address,
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
