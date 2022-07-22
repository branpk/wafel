use fast3d::{
    decode::{decode_f3d_display_list, F3DCommandIter, RawF3DCommand},
    interpret::{interpret_f3d_display_list, F3DMemory, F3DRenderData},
};
use wafel_api::{Address, Error};
use wafel_data_access::MemoryLayout;
use wafel_memory::{MemoryError, MemoryRead};

pub fn render_sm64_dl(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
    screen_size: (u32, u32),
) -> Result<F3DRenderData, Error> {
    if let Some(root_addr) = get_dl_addr(layout, memory)? {
        let f3d_memory = F3DMemoryImpl { memory, root_addr };
        let render_data = interpret_f3d_display_list(&f3d_memory, screen_size, true)?;

        Ok(render_data)
    } else {
        Ok(F3DRenderData::default())
    }
}

fn get_dl_addr(
    layout: &impl MemoryLayout,
    memory: &impl MemoryRead,
) -> Result<Option<Address>, Error> {
    let addr = layout.global_path("gGfxPool?")?.read(memory)?;
    if addr.is_none() {
        Ok(None)
    } else {
        Ok(Some(addr.try_as_address()?))
    }
}

#[derive(Debug)]
pub struct F3DMemoryImpl<'m, M> {
    memory: &'m M,
    root_addr: Address,
}

impl<'m, M: MemoryRead> F3DMemoryImpl<'m, M> {
    fn read_dl_impl(&self, addr: Address) -> <Self as F3DMemory>::DlIter {
        let raw = RawDlIter {
            memory: self.memory,
            addr,
        };
        decode_f3d_display_list(raw)
    }
}

impl<'m, M: MemoryRead> F3DMemory for F3DMemoryImpl<'m, M> {
    type Ptr = Address;
    type Error = MemoryError;
    type DlIter = F3DCommandIter<RawDlIter<'m, M>>;

    fn root_dl(&self) -> Result<Self::DlIter, Self::Error> {
        Ok(self.read_dl_impl(self.root_addr))
    }

    fn read_dl(&self, ptr: Self::Ptr) -> Result<Self::DlIter, Self::Error> {
        Ok(self.read_dl_impl(ptr))
    }

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        self.memory.read_u8s(ptr + offset, dst)
    }

    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        self.memory.read_u16s(ptr + offset, dst)
    }

    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        self.memory.read_u32s(ptr + offset, dst)
    }
}

#[derive(Debug)]
pub struct RawDlIter<'m, M> {
    pub memory: &'m M,
    pub addr: Address,
}

impl<'m, M: MemoryRead> RawDlIter<'m, M> {
    fn next_impl(&mut self) -> Result<RawF3DCommand<Address>, MemoryError> {
        let w_type = self.memory.pointer_int_type();
        let w_size = w_type.size();

        let w0 = self.memory.read_int(self.addr, w_type)? as u32;
        self.addr += w_size;

        let w1 = self.memory.read_int(self.addr, w_type)? as u32;
        let w1_ptr = self.memory.read_addr(self.addr)?;
        self.addr += w_size;

        Ok(RawF3DCommand { w0, w1, w1_ptr })
    }
}

impl<'m, M: MemoryRead> Iterator for RawDlIter<'m, M> {
    type Item = Result<RawF3DCommand<Address>, MemoryError>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_impl())
    }
}
