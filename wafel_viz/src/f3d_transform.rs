use std::sync::Arc;

use fast3d::{
    decode::{decode_f3d_display_list, F3DCommandIter, RawF3DCommand},
    interpret::{interpret_f3d_display_list, F3DMemory, F3DRenderData},
};
use wafel_api::{Address, Error, IntType};
use wafel_data_path::GlobalDataPath;
use wafel_data_type::DataType;
use wafel_memory::{MemoryError, MemoryRead};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SM64RenderConfig {
    pub screen_size: Option<(u32, u32)>,
}

pub fn process_display_list(
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

    let f3d_memory = F3DMemoryImpl {
        memory,
        root_addr: dl_addr,
    };
    let render_data =
        interpret_f3d_display_list(&f3d_memory, config.screen_size.unwrap_or((320, 240)), true)?;

    Ok(render_data)
}

#[derive(Debug)]
struct F3DMemoryImpl<'m, M> {
    memory: &'m M,
    root_addr: Address,
}

impl<'m, M: MemoryRead> F3DMemory for F3DMemoryImpl<'m, M> {
    type Ptr = Address;
    type Error = MemoryError;
    type DlIter = F3DCommandIter<RawDlIter<'m, M>>;

    fn root_dl(&self) -> Result<Self::DlIter, Self::Error> {
        self.read_dl(self.root_addr)
    }

    fn read_dl(&self, ptr: Self::Ptr) -> Result<Self::DlIter, Self::Error> {
        Ok(decode_f3d_display_list(RawDlIter {
            memory: self.memory,
            addr: ptr,
        }))
    }

    // TODO: Optimize buffer reads?

    fn read_u8(&self, dst: &mut [u8], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr + offset;
        for i in 0..dst.len() {
            dst[i] = self.memory.read_int(addr + i, IntType::U8)? as u8;
        }
        Ok(())
    }

    fn read_u16(&self, dst: &mut [u16], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr + offset;
        for i in 0..dst.len() {
            dst[i] = self.memory.read_int(addr + 2 * i, IntType::U16)? as u16;
        }
        Ok(())
    }

    fn read_u32(&self, dst: &mut [u32], ptr: Self::Ptr, offset: usize) -> Result<(), Self::Error> {
        let addr = ptr + offset;
        for i in 0..dst.len() {
            dst[i] = self.memory.read_int(addr + 4 * i, IntType::U32)? as u32;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct RawDlIter<'m, M> {
    memory: &'m M,
    addr: Address,
}

impl<'m, M: MemoryRead> RawDlIter<'m, M> {
    fn next_impl(&mut self) -> Result<RawF3DCommand<Address>, MemoryError> {
        let w_type = self.memory.pointer_int_type();
        let w_size = w_type.size();

        let w0 = self.memory.read_int(self.addr, w_type)? as u32;
        self.addr += w_size;

        let w1 = self.memory.read_int(self.addr, w_type)? as u32;
        let w1_ptr = self.memory.read_address(self.addr)?;
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
