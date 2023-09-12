use std::{
    mem,
    sync::Mutex,
    time::{Duration, Instant},
};

use bytemuck::Pod;
use once_cell::sync::Lazy;
use process_memory::{CopyAddress, Pid, ProcessHandle, PutAddress, TryIntoProcessHandle};
use sysinfo::{PidExt, ProcessRefreshKind, RefreshKind, System, SystemExt};
use wafel_data_type::{Address, IntType};
use wafel_layout::{append_dll_extension, read_dll_segments};

use crate::{
    MemoryError::{self, *},
    MemoryInitError, MemoryRead, MemoryWrite,
};

// TODO: Detect when process closes

#[derive(Debug, Clone)]
struct ProcessHandleWrapper(ProcessHandle);

unsafe impl Sync for ProcessHandleWrapper {}
unsafe impl Send for ProcessHandleWrapper {}

fn is_process_open(pid: u32) -> bool {
    static SYSTEM: Lazy<Mutex<System>> = Lazy::new(|| {
        Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::new()),
        ))
    });
    let mut system = SYSTEM.lock().unwrap();
    system.refresh_process_specifics(sysinfo::Pid::from_u32(pid), ProcessRefreshKind::new())
}

/// Memory view for reading/writing into an libsm64 instance running in a different
/// process.
///
/// This uses the same ownership model as [EmuMemory](crate::EmuMemory),
/// namely that it has no ownership or unique access to the target DLL's memory,
/// and can therefore be cloned.
/// Note that this is different from [DllGameMemory](crate::DllGameMemory) which
/// does have ownership.
///
/// For now, [GameMemory](crate::GameMemory) is not implemented for simplicity,
/// but it could be in the future. (We may want to rethink that abstraction though.)
#[derive(Debug, Clone)]
pub struct RemoteDllMemory {
    pid: u32,
    handle: ProcessHandleWrapper,
    base_address: usize,
    base_size: usize,
    cache: Option<Vec<u8>>,
}

impl RemoteDllMemory {
    /// Attach to a running process which has loaded the given DLL and return a
    /// [RemoteDllMemory] representing a read/write view of the DLL's memory.
    pub fn attach(pid: u32, base_address: usize, dll_path: &str) -> Result<Self, MemoryInitError> {
        let handle = (pid as Pid)
            .try_into_process_handle()
            .map_err(|error| MemoryInitError::ProcessAttachError(error.into()))?;

        let dll_path = append_dll_extension(dll_path);

        let all_segments = read_dll_segments(dll_path)?;

        let base_size = all_segments
            .iter()
            .map(|segment| (segment.virtual_address + segment.virtual_size) as usize)
            .max()
            .unwrap_or(0);

        Ok(Self {
            pid,
            handle: ProcessHandleWrapper(handle),
            base_address,
            base_size,
            cache: None,
        })
    }

    /// Return true if a process with the given pid is currently open.
    ///
    /// If the process is closed, then reads and writes on this memory object
    /// will fail. Once this method returns false, you should avoid using this
    /// memory again since a new process may eventually open with the same pid.
    ///
    /// Note that a process may close immediately after calling this method,
    /// so failed reads/writes must be handled regardless.
    pub fn is_process_open(&self) -> bool {
        is_process_open(self.pid)
    }

    pub fn load_cache(&mut self, global_timer_addr: Address) -> Result<(), MemoryError> {
        self.sync_to_game(global_timer_addr)?;

        let cache = self.cache.get_or_insert_with(|| vec![0; self.base_size]);
        self.handle
            .0
            .copy_address(self.base_address, cache)
            .map_err(|error| ProcessReadError(error.into()))?;

        Ok(())
    }

    fn sync_to_game(&self, global_timer_addr: Address) -> Result<(), MemoryError> {
        let offset = global_timer_addr.0;
        self.validate_offset::<u8>(offset, 4, self.base_size)?;
        let process_addr = self.base_address + offset;

        let mut initial: [u8; 4] = Default::default();
        self.read_bytes_uncached(process_addr, &mut initial)?;

        let start = Instant::now();
        while start.elapsed() < Duration::from_secs_f32(1.0 / 30.0) {
            let mut current: [u8; 4] = Default::default();
            self.read_bytes_uncached(process_addr, &mut current)?;

            if initial != current {
                break;
            }
        }
        Ok(())
    }

    fn validate_offset<T>(
        &self,
        offset: usize,
        len: usize,
        range_size: usize,
    ) -> Result<(), MemoryError> {
        let data_size = mem::size_of::<T>()
            .checked_mul(len)
            .expect("buffer is too large");
        if offset + data_size > range_size {
            Err(InvalidAddress)
        } else {
            Ok(())
        }
    }

    /// Translate the pointer in the dll to a relocatable address.
    fn unchecked_pointer_value_to_address(&self, pointer: usize) -> Address {
        if pointer == 0 {
            Address::NULL
        } else {
            let offset = pointer.wrapping_sub(self.base_address);
            Address(offset)
        }
    }

    fn unchecked_address_to_pointer_value(&self, address: Address) -> usize {
        if address.is_null() {
            0
        } else {
            self.base_address.wrapping_add(address.0)
        }
    }

    /// Read bytes from the target process without validation and bypassing the cache.
    fn read_bytes_uncached(
        &self,
        process_addr: usize,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        self.handle
            .0
            .copy_address(process_addr, buffer)
            .map_err(|error| ProcessReadError(error.into()))?;
        Ok(())
    }

    fn read_bytes(&self, addr: Address, buffer: &mut [u8]) -> Result<(), MemoryError> {
        let offset = addr.0;
        self.validate_offset::<u8>(offset, buffer.len(), self.base_size)?;
        match &self.cache {
            Some(cache) => {
                buffer.copy_from_slice(&cache[offset..offset + buffer.len()]);
            }
            None => {
                self.read_bytes_uncached(self.base_address + offset, buffer)?;
            }
        }
        Ok(())
    }

    fn write_bytes(&mut self, addr: Address, buffer: &[u8]) -> Result<(), MemoryError> {
        let offset = addr.0;
        self.validate_offset::<u8>(offset, buffer.len(), self.base_size)?;
        self.handle
            .0
            .put_address(self.base_address + offset, buffer)
            .map_err(|error| ProcessReadError(error.into()))?;
        if let Some(cache) = &mut self.cache {
            cache[offset..offset + buffer.len()].copy_from_slice(buffer);
        }
        Ok(())
    }

    fn read_buffer<T: Pod>(&self, addr: Address, buffer: &mut [T]) -> Result<(), MemoryError> {
        self.read_bytes(addr, bytemuck::cast_slice_mut(buffer))?;
        Ok(())
    }

    fn write_buffer<T: Pod>(&mut self, addr: Address, buffer: &[T]) -> Result<(), MemoryError> {
        self.write_bytes(addr, bytemuck::cast_slice(buffer))?;
        Ok(())
    }
}

impl MemoryRead for RemoteDllMemory {
    fn read_u8s(&self, addr: Address, buf: &mut [u8]) -> Result<(), MemoryError> {
        self.read_buffer(addr, buf)
    }

    fn read_u16s(&self, addr: Address, buf: &mut [u16]) -> Result<(), MemoryError> {
        self.read_buffer(addr, buf)
    }

    fn read_u32s(&self, addr: Address, buf: &mut [u32]) -> Result<(), MemoryError> {
        self.read_buffer(addr, buf)
    }

    fn read_u64s(&self, addr: Address, buf: &mut [u64]) -> Result<(), MemoryError> {
        self.read_buffer(addr, buf)
    }

    fn read_addr(&self, addr: Address) -> Result<Address, MemoryError> {
        let mut pointers: [usize; 1] = [0];
        self.read_buffer(addr, &mut pointers)?;
        Ok(self.unchecked_pointer_value_to_address(pointers[0]))
    }

    fn read_addrs(&self, addr: Address, buf: &mut [Address]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter_mut().enumerate() {
            *value = self.read_addr(addr + mem::size_of::<usize>() * i)?;
        }
        Ok(())
    }

    fn pointer_int_type(&self) -> IntType {
        IntType::u_ptr_native()
    }
}

impl MemoryWrite for RemoteDllMemory {
    fn write_u8s(&mut self, addr: Address, buf: &[u8]) -> Result<(), MemoryError> {
        self.write_buffer(addr, buf)
    }

    fn write_u16s(&mut self, addr: Address, buf: &[u16]) -> Result<(), MemoryError> {
        self.write_buffer(addr, buf)
    }

    fn write_u32s(&mut self, addr: Address, buf: &[u32]) -> Result<(), MemoryError> {
        self.write_buffer(addr, buf)
    }

    fn write_u64s(&mut self, addr: Address, buf: &[u64]) -> Result<(), MemoryError> {
        self.write_buffer(addr, buf)
    }

    fn write_addr(&mut self, addr: Address, value: Address) -> Result<(), MemoryError> {
        let pointer = self.unchecked_address_to_pointer_value(value);
        self.write_buffer(addr, &[pointer])
    }

    fn write_addrs(&mut self, addr: Address, buf: &[Address]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().enumerate() {
            self.write_addr(addr + mem::size_of::<usize>() * i, *value)?;
        }
        Ok(())
    }
}
