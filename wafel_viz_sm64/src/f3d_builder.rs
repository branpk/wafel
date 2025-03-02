use core::{fmt, slice};
use std::iter::Peekable;

use fast3d::{
    cmd::F3DCommand,
    decode::{decode_f3d_display_list, F3DCommandIter, RawF3DCommand},
    interpret::F3DMemory,
    util::Vertex,
};
use wafel_data_type::Address;
use wafel_memory::MemoryRead;

use crate::error::VizError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pointer {
    Segmented(Segmented),
    BufferOffset(usize),
}

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pointer::Segmented(seg) => write!(f, "Segmented({})", seg.0),
            Pointer::BufferOffset(offset) => write!(f, "BufferOffset({})", offset),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Segmented(pub Address);

impl Pointer {
    pub fn segmented(self) -> Option<Segmented> {
        match self {
            Pointer::Segmented(segmented) => Some(segmented),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct F3DBuilder<'m, M: MemoryRead> {
    memory: &'m M,
    seg_table: Option<Vec<u32>>,
    input_dl: Peekable<F3DCommandIter<RawDlIter<'m, M>>>,
    root_dl: Vec<F3DCommand<Pointer>>,
    dl_buffer: Vec<F3DCommand<Pointer>>,
    u32_buffer: Vec<u32>,
    vertex_buffer: Vec<Vertex>,
}

impl<'m, M: MemoryRead> F3DBuilder<'m, M> {
    pub fn new(memory: &'m M, input_dl_addr: Address, seg_table: Option<Vec<u32>>) -> Self {
        let input_dl = decode_f3d_display_list(RawDlIter {
            memory,
            addr: input_dl_addr,
        })
        .peekable();
        Self {
            memory,
            seg_table,
            input_dl,
            root_dl: Vec::new(),
            dl_buffer: Vec::new(),
            u32_buffer: Vec::new(),
            vertex_buffer: Vec::new(),
        }
    }

    pub fn push_cmd(&mut self, cmd: F3DCommand<Pointer>) {
        self.root_dl.push(cmd);
    }

    pub fn push_until(
        &mut self,
        mut f: impl FnMut(F3DCommand<Pointer>) -> bool,
    ) -> Result<bool, VizError> {
        loop {
            match self.input_dl.peek().cloned() {
                Some(cmd) => {
                    let cmd = cmd?;
                    if f(cmd) {
                        return Ok(true);
                    } else {
                        self.push_cmd(cmd);
                        self.input_dl.next();
                    }
                }
                None => return Ok(false),
            }
        }
    }

    pub fn expect(
        &mut self,
        mut f: impl FnMut(F3DCommand<Pointer>) -> bool,
    ) -> Result<F3DCommand<Pointer>, VizError> {
        if let Some(cmd) = self.input_dl.peek() {
            let cmd = cmd.clone()?;
            if f(cmd) {
                self.input_dl.next();
                return Ok(cmd);
            } else {
                // panic!("{:?}", cmd);
            }
        }
        Err(VizError::UnexpectedDisplayListCommand)
    }

    pub fn push_expect(
        &mut self,
        f: impl FnMut(F3DCommand<Pointer>) -> bool,
    ) -> Result<(), VizError> {
        let cmd = self.expect(f)?;
        self.push_cmd(cmd);
        Ok(())
    }

    pub fn push_remaining(&mut self) -> Result<(), VizError> {
        while let Some(cmd) = self.input_dl.next() {
            let cmd = cmd?;
            self.push_cmd(cmd);
        }
        Ok(())
    }

    pub fn alloc_dl(&mut self, buf: &[F3DCommand<Pointer>]) -> Pointer {
        let ptr = Pointer::BufferOffset(self.dl_buffer.len());
        self.dl_buffer.extend(buf);
        ptr
    }

    pub fn alloc_u32(&mut self, buf: &[u32]) -> Pointer {
        let ptr = Pointer::BufferOffset(self.u32_buffer.len());
        self.u32_buffer.extend(buf);
        ptr
    }

    pub fn alloc_vertices(&mut self, buf: &[Vertex]) -> Pointer {
        let ptr = Pointer::BufferOffset(self.vertex_buffer.len());
        self.vertex_buffer.extend(buf);
        ptr
    }

    pub fn seg_to_virt(&self, segmented: Segmented) -> Address {
        if let Some(seg_table) = &self.seg_table {
            let addr = (segmented.0).0 as u32;
            let segment = (addr & 0x1FFF_FFFF) >> 24;
            let offset = addr & 0x00FF_FFFF;

            let base = seg_table[segment as usize] | 0x8000_0000;
            let base = Address(base as usize);

            base + offset as usize
        } else {
            segmented.0
        }
    }

    pub fn virt_to_phys(&self, addr: Address) -> Segmented {
        if self.seg_table.is_some() {
            Segmented(Address(((addr.0 as u32) & 0x1FFF_FFFF) as usize))
        } else {
            Segmented(addr)
        }
    }
}

impl<'m, M: MemoryRead> F3DMemory for F3DBuilder<'m, M> {
    type Ptr = Pointer;
    type Error = VizError;
    type DlIter<'a> = DlIter<'a, 'm, M> where Self: 'a;

    fn root_dl(&self) -> Result<Self::DlIter<'_>, Self::Error> {
        Ok(DlIter::FromBuffer(self.root_dl.iter()))
    }

    fn read_dl(&self, ptr: Self::Ptr) -> Result<Self::DlIter<'_>, Self::Error> {
        match ptr {
            Pointer::Segmented(seg) => {
                let addr = self.seg_to_virt(seg);
                let raw = RawDlIter {
                    memory: self.memory,
                    addr,
                };
                Ok(DlIter::FromRaw(decode_f3d_display_list(raw)))
            }
            Pointer::BufferOffset(offset) => {
                Ok(DlIter::FromBuffer(self.dl_buffer[offset..].iter()))
            }
        }
    }

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = self.seg_to_virt(ptr.segmented().ok_or(VizError::InvalidF3DPointer)?);
        self.memory.read_u8s(addr + offset, dst)?;
        Ok(())
    }

    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = self.seg_to_virt(ptr.segmented().ok_or(VizError::InvalidF3DPointer)?);
        self.memory.read_u16s(addr + offset, dst)?;
        Ok(())
    }

    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        match ptr {
            Pointer::Segmented(segmented) => {
                self.memory
                    .read_u32s(self.seg_to_virt(segmented) + offset, dst)?;
            }
            Pointer::BufferOffset(offset) => {
                dst.copy_from_slice(&self.u32_buffer[offset..offset + dst.len()]);
            }
        }
        Ok(())
    }

    fn read_vertices(
        &self,
        dst: &mut [Vertex],
        ptr: Self::Ptr,
        offset: usize,
    ) -> Result<(), Self::Error> {
        match ptr {
            Pointer::Segmented(_) => self.read_vertices_default(dst, ptr, offset)?,
            Pointer::BufferOffset(offset) => {
                dst.copy_from_slice(&self.vertex_buffer[offset..offset + dst.len()]);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum DlIter<'a, 'm, M> {
    FromBuffer(slice::Iter<'a, F3DCommand<Pointer>>),
    FromRaw(F3DCommandIter<RawDlIter<'m, M>>),
}

impl<'a, 'm, M: MemoryRead> Iterator for DlIter<'a, 'm, M> {
    type Item = Result<F3DCommand<Pointer>, VizError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DlIter::FromBuffer(iter) => iter.next().copied().map(Ok),
            DlIter::FromRaw(iter) => iter.next(),
        }
    }
}

#[derive(Debug)]
pub struct RawDlIter<'m, M> {
    memory: &'m M,
    addr: Address,
}

impl<'m, M: MemoryRead> RawDlIter<'m, M> {
    fn next_impl(&mut self) -> Result<RawF3DCommand<Pointer>, VizError> {
        let w_type = self.memory.pointer_int_type();
        let w_size = w_type.size();

        let w0 = self.memory.read_int(self.addr, w_type)? as u32;
        self.addr += w_size;

        let w1 = self.memory.read_int(self.addr, w_type)? as u32;
        let w1_ptr = Pointer::Segmented(Segmented(self.memory.read_addr(self.addr)?));
        self.addr += w_size;

        Ok(RawF3DCommand { w0, w1, w1_ptr })
    }
}

impl<'m, M: MemoryRead> Iterator for RawDlIter<'m, M> {
    type Item = Result<RawF3DCommand<Pointer>, VizError>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_impl())
    }
}
