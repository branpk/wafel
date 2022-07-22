use core::slice;
use std::iter::Peekable;

use fast3d::{
    cmd::F3DCommand,
    decode::{decode_f3d_display_list, F3DCommandIter, RawF3DCommand},
    interpret::F3DMemory,
};
use wafel_data_type::Address;
use wafel_memory::MemoryRead;

use crate::error::VizError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pointer {
    Address(Address),
    BufferOffset(usize),
}

impl From<Address> for Pointer {
    fn from(v: Address) -> Self {
        Self::Address(v)
    }
}

impl Pointer {
    pub fn addr(self) -> Option<Address> {
        match self {
            Pointer::Address(addr) => Some(addr),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct F3DBuilder<'m, M: MemoryRead> {
    memory: &'m M,
    input_dl: Peekable<F3DCommandIter<RawDlIter<'m, M>>>,
    dl_buffer: Vec<F3DCommand<Pointer>>,
    u32_buffer: Vec<u32>,
}

impl<'m, M: MemoryRead> F3DBuilder<'m, M> {
    pub fn new(memory: &'m M, input_dl_addr: Address) -> Self {
        let input_dl = decode_f3d_display_list(RawDlIter {
            memory,
            addr: input_dl_addr,
        })
        .peekable();
        Self {
            memory,
            input_dl,
            dl_buffer: Vec::new(),
            u32_buffer: Vec::new(),
        }
    }

    pub fn push_cmd(&mut self, cmd: F3DCommand<Pointer>) {
        self.dl_buffer.push(cmd);
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

    pub fn alloc_u32(&mut self, buf: &[u32]) -> Pointer {
        let ptr = Pointer::BufferOffset(self.u32_buffer.len());
        self.u32_buffer.extend(buf);
        ptr
    }

    pub fn read_u32(&self, dst: &mut [u32], ptr: Pointer, offset: usize) -> Result<(), VizError> {
        match ptr {
            Pointer::Address(addr) => {
                self.memory.read_u32s(addr + offset, dst)?;
            }
            Pointer::BufferOffset(offset) => {
                dst.copy_from_slice(&self.u32_buffer[offset..offset + dst.len()]);
            }
        }
        Ok(())
    }
}

impl<'m, M: MemoryRead> F3DMemory for F3DBuilder<'m, M> {
    type Ptr = Pointer;
    type Error = VizError;
    type DlIter<'a> = DlIter<'a, 'm, M> where Self: 'a;

    fn root_dl(&self) -> Result<Self::DlIter<'_>, Self::Error> {
        Ok(DlIter::FromBuffer(self.dl_buffer.iter()))
    }

    fn read_dl(&self, ptr: Self::Ptr) -> Result<Self::DlIter<'_>, Self::Error> {
        let addr = ptr.addr().expect("invalid display list pointer");
        let raw = RawDlIter {
            memory: self.memory,
            addr,
        };
        Ok(DlIter::FromRaw(decode_f3d_display_list(raw)))
    }

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr.addr().expect("invalid u8 pointer");
        self.memory.read_u8s(addr + offset, dst)?;
        Ok(())
    }

    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr.addr().expect("invalid u16 pointer");
        self.memory.read_u16s(addr + offset, dst)?;
        Ok(())
    }

    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        match ptr {
            Pointer::Address(addr) => {
                self.memory.read_u32s(addr + offset, dst)?;
            }
            Pointer::BufferOffset(offset) => {
                dst.copy_from_slice(&self.u32_buffer[offset..offset + dst.len()]);
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
    pub memory: &'m M,
    pub addr: Address,
}

impl<'m, M: MemoryRead> RawDlIter<'m, M> {
    fn next_impl(&mut self) -> Result<RawF3DCommand<Pointer>, VizError> {
        let w_type = self.memory.pointer_int_type();
        let w_size = w_type.size();

        let w0 = self.memory.read_int(self.addr, w_type)? as u32;
        self.addr += w_size;

        let w1 = self.memory.read_int(self.addr, w_type)? as u32;
        let w1_ptr = Pointer::Address(self.memory.read_addr(self.addr)?);
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
