use std::time::{Duration, Instant};

use process_memory::{CopyAddress, Pid, ProcessHandle, PutAddress, TryIntoProcessHandle};
use wafel_data_type::{Address, IntType};

use crate::{
    is_process_open,
    MemoryError::{self, *},
    MemoryInitError, MemoryRead, MemoryWrite,
};

// TODO: Cache hints, e.g. in read_surfaces
// TODO: More efficient buffer reading/writing

#[derive(Debug, Clone)]
struct ProcessHandleWrapper(ProcessHandle);

unsafe impl Sync for ProcessHandleWrapper {}
unsafe impl Send for ProcessHandleWrapper {}

// EmuMemory doesn't implement GameMemory because it isn't able to make any guarantees about
// how/when the process writes to the base slot.
// In the future, Wafel could have an embedded emulator that it can control, which
// could implement GameMemory.

/// Memory view for reading/writing to a running emulator.
///
/// EmuMemory should be thought of as using interior mutability, since it has no ownership or
/// unique access to the process's memory.
/// The [MemoryWrite] trait takes &mut self, but the `EmuMemory` object can be cloned
/// when needed.
#[derive(Debug, Clone)]
pub struct EmuMemory {
    pid: u32,
    handle: ProcessHandleWrapper,
    base_address: usize,
    memory_size: usize,
    cache: Option<Vec<u8>>,
}

impl EmuMemory {
    /// Attach to a running emulator and return an [EmuMemory] representing a read/write view
    /// of the process's memory.
    pub fn attach(
        pid: u32,
        base_address: usize,
        memory_size: usize,
    ) -> Result<Self, MemoryInitError> {
        let handle = (pid as Pid)
            .try_into_process_handle()
            .map_err(|error| MemoryInitError::ProcessAttachError(error.into()))?;

        Ok(Self {
            pid,
            handle: ProcessHandleWrapper(handle),
            base_address,
            memory_size,
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

        let cache = self.cache.get_or_insert_with(|| vec![0; self.memory_size]);
        self.handle
            .0
            .copy_address(self.base_address, cache)
            .map_err(|error| ProcessReadError(error.into()))?;

        Ok(())
    }

    fn sync_to_game(&self, global_timer_addr: Address) -> Result<(), MemoryError> {
        let process_addr = self.base_address + self.validate_offset(global_timer_addr, 4)?;

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

    fn validate_offset(&self, addr: Address, size: usize) -> Result<usize, MemoryError> {
        let offset = addr.0 & 0x3FFF_FFFF;
        if offset + size > self.memory_size {
            Err(InvalidAddress)
        } else {
            Ok(offset)
        }
    }

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
        let offset = self.validate_offset(addr, buffer.len())?;
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
        let offset = self.validate_offset(addr, buffer.len())?;
        self.handle
            .0
            .put_address(self.base_address + offset, buffer)
            .map_err(|error| ProcessReadError(error.into()))?;
        if let Some(cache) = &mut self.cache {
            cache[offset..offset + buffer.len()].copy_from_slice(buffer);
        }
        Ok(())
    }

    fn check_align(&self, addr: Address, align: usize) -> Result<(), MemoryError> {
        if addr.0 % align == 0 {
            Ok(())
        } else {
            Err(InvalidAddress)
        }
    }

    fn swap_1(&self, addr: Address) -> Address {
        if cfg!(target_endian = "big") {
            addr
        } else {
            let truncated = addr.0 - addr.0 % 4;
            Address(truncated + 3 - addr.0 % 4)
        }
    }

    fn read_u8(&self, addr: Address) -> Result<u8, MemoryError> {
        self.check_align(addr, 1)?;
        let addr = self.swap_1(addr);
        let mut bytes = [0u8; 1];
        self.read_bytes(addr, &mut bytes)?;
        Ok(u8::from_ne_bytes(bytes))
    }

    fn write_u8(&mut self, addr: Address, value: u8) -> Result<(), MemoryError> {
        self.check_align(addr, 1)?;
        let addr = self.swap_1(addr);
        let bytes = value.to_ne_bytes();
        self.write_bytes(addr, &bytes)?;
        Ok(())
    }

    fn swap_2(&self, addr: Address) -> Address {
        if cfg!(target_endian = "big") {
            addr
        } else {
            let truncated = addr.0 - addr.0 % 4;
            Address(truncated + 2 - addr.0 % 4)
        }
    }

    fn read_u16(&self, addr: Address) -> Result<u16, MemoryError> {
        self.check_align(addr, 2)?;
        let addr = self.swap_2(addr);
        let mut bytes = [0u8; 2];
        self.read_bytes(addr, &mut bytes)?;
        Ok(u16::from_ne_bytes(bytes))
    }

    fn write_u16(&mut self, addr: Address, value: u16) -> Result<(), MemoryError> {
        self.check_align(addr, 2)?;
        let addr = self.swap_2(addr);
        let bytes = value.to_ne_bytes();
        self.write_bytes(addr, &bytes)?;
        Ok(())
    }

    fn read_u32(&self, addr: Address) -> Result<u32, MemoryError> {
        self.check_align(addr, 4)?;
        let mut bytes = [0u8; 4];
        self.read_bytes(addr, &mut bytes)?;
        Ok(u32::from_ne_bytes(bytes))
    }

    fn write_u32(&mut self, addr: Address, value: u32) -> Result<(), MemoryError> {
        self.check_align(addr, 4)?;
        let bytes = value.to_ne_bytes();
        self.write_bytes(addr, &bytes)?;
        Ok(())
    }

    fn read_u64(&self, addr: Address) -> Result<u64, MemoryError> {
        self.check_align(addr, 4)?;
        let mut bytes = [0u8; 8];
        self.read_bytes(addr, &mut bytes)?;
        let upper = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let lower = u32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        Ok((upper as u64) << 32 | lower as u64)
    }

    fn write_u64(&mut self, addr: Address, value: u64) -> Result<(), MemoryError> {
        self.check_align(addr, 4)?;
        let upper = ((value >> 32) as u32).to_ne_bytes();
        let lower = (value as u32).to_ne_bytes();
        let bytes = [
            upper[0], upper[1], upper[2], upper[3], lower[0], lower[1], lower[2], lower[3],
        ];
        self.write_bytes(addr, &bytes)?;
        Ok(())
    }
}

impl MemoryRead for EmuMemory {
    fn read_u8s(&self, addr: Address, buf: &mut [u8]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter_mut().enumerate() {
            *value = self.read_u8(addr + i)?;
        }
        Ok(())
    }

    fn read_u16s(&self, addr: Address, buf: &mut [u16]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter_mut().enumerate() {
            *value = self.read_u16(addr + 2 * i)?;
        }
        Ok(())
    }

    fn read_u32s(&self, addr: Address, buf: &mut [u32]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter_mut().enumerate() {
            *value = self.read_u32(addr + 4 * i)?;
        }
        Ok(())
    }

    fn read_u64s(&self, addr: Address, buf: &mut [u64]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter_mut().enumerate() {
            *value = self.read_u64(addr + 8 * i)?;
        }
        Ok(())
    }

    fn read_addrs(&self, addr: Address, buf: &mut [Address]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter_mut().enumerate() {
            *value = Address(self.read_u32(addr + 4 * i)? as usize);
        }
        Ok(())
    }

    fn pointer_int_type(&self) -> IntType {
        IntType::U32
    }
}

impl MemoryWrite for EmuMemory {
    fn write_u8s(&mut self, addr: Address, buf: &[u8]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            self.write_u8(addr + i, value)?;
        }
        Ok(())
    }

    fn write_u16s(&mut self, addr: Address, buf: &[u16]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            self.write_u16(addr + 2 * i, value)?;
        }
        Ok(())
    }

    fn write_u32s(&mut self, addr: Address, buf: &[u32]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            self.write_u32(addr + 4 * i, value)?;
        }
        Ok(())
    }

    fn write_u64s(&mut self, addr: Address, buf: &[u64]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            self.write_u64(addr + 8 * i, value)?;
        }
        Ok(())
    }

    fn write_addrs(&mut self, addr: Address, buf: &[Address]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            self.write_u32(addr + 4 * i, value.0 as u32)?;
        }
        Ok(())
    }
}
