use process_memory::{CopyAddress, Pid, ProcessHandle, PutAddress, TryIntoProcessHandle};
use wafel_data_type::{Address, IntType};

use crate::{
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
    handle: ProcessHandleWrapper,
    base_address: usize,
    memory_size: usize,
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
            handle: ProcessHandleWrapper(handle),
            base_address,
            memory_size,
        })
    }

    fn validate_address(&self, address: Address, size: usize) -> Result<usize, MemoryError> {
        let offset = address.0 & 0x3FFF_FFFF;
        if offset + size > self.memory_size {
            Err(InvalidAddress)
        } else {
            Ok(self.base_address + offset)
        }
    }

    fn read_bytes(&self, address: Address, buffer: &mut [u8]) -> Result<(), MemoryError> {
        let process_address = self.validate_address(address, buffer.len())?;
        self.handle
            .0
            .copy_address(process_address, buffer)
            .map_err(|error| ProcessReadError(error.into()))?;
        Ok(())
    }

    fn write_bytes(&self, address: Address, buffer: &[u8]) -> Result<(), MemoryError> {
        let process_address = self.validate_address(address, buffer.len())?;
        self.handle
            .0
            .put_address(process_address, buffer)
            .map_err(|error| ProcessReadError(error.into()))?;
        Ok(())
    }

    fn check_align(&self, address: Address, align: usize) -> Result<(), MemoryError> {
        if address.0 % align == 0 {
            Ok(())
        } else {
            Err(InvalidAddress)
        }
    }

    fn swap_1(&self, address: Address) -> Address {
        if cfg!(target_endian = "big") {
            address
        } else {
            let truncated = address.0 - address.0 % 4;
            Address(truncated + 3 - address.0 % 4)
        }
    }

    fn read_u8(&self, address: Address) -> Result<u8, MemoryError> {
        self.check_align(address, 1)?;
        let address = self.swap_1(address);
        let mut bytes = [0u8; 1];
        self.read_bytes(address, &mut bytes)?;
        Ok(u8::from_ne_bytes(bytes))
    }

    fn write_u8(&self, address: Address, value: u8) -> Result<(), MemoryError> {
        self.check_align(address, 1)?;
        let address = self.swap_1(address);
        let bytes = value.to_ne_bytes();
        self.write_bytes(address, &bytes)?;
        Ok(())
    }

    fn swap_2(&self, address: Address) -> Address {
        if cfg!(target_endian = "big") {
            address
        } else {
            let truncated = address.0 - address.0 % 4;
            Address(truncated + 2 - address.0 % 4)
        }
    }

    fn read_u16(&self, address: Address) -> Result<u16, MemoryError> {
        self.check_align(address, 2)?;
        let address = self.swap_2(address);
        let mut bytes = [0u8; 2];
        self.read_bytes(address, &mut bytes)?;
        Ok(u16::from_ne_bytes(bytes))
    }

    fn write_u16(&self, address: Address, value: u16) -> Result<(), MemoryError> {
        self.check_align(address, 2)?;
        let address = self.swap_2(address);
        let bytes = value.to_ne_bytes();
        self.write_bytes(address, &bytes)?;
        Ok(())
    }

    fn read_u32(&self, address: Address) -> Result<u32, MemoryError> {
        self.check_align(address, 4)?;
        let mut bytes = [0u8; 4];
        self.read_bytes(address, &mut bytes)?;
        Ok(u32::from_ne_bytes(bytes))
    }

    fn write_u32(&self, address: Address, value: u32) -> Result<(), MemoryError> {
        self.check_align(address, 4)?;
        let bytes = value.to_ne_bytes();
        self.write_bytes(address, &bytes)?;
        Ok(())
    }

    fn read_u64(&self, address: Address) -> Result<u64, MemoryError> {
        self.check_align(address, 4)?;
        let mut bytes = [0u8; 8];
        self.read_bytes(address, &mut bytes)?;
        let upper = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let lower = u32::from_ne_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        Ok((upper as u64) << 32 | lower as u64)
    }

    fn write_u64(&self, address: Address, value: u64) -> Result<(), MemoryError> {
        self.check_align(address, 4)?;
        let upper = ((value >> 32) as u32).to_ne_bytes();
        let lower = (value as u32).to_ne_bytes();
        let bytes = [
            upper[0], upper[1], upper[2], upper[3], lower[0], lower[1], lower[2], lower[3],
        ];
        self.write_bytes(address, &bytes)?;
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
            (&*self).write_u8(addr + i, value)?;
        }
        Ok(())
    }

    fn write_u16s(&mut self, addr: Address, buf: &[u16]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            (&*self).write_u16(addr + 2 * i, value)?;
        }
        Ok(())
    }

    fn write_u32s(&mut self, addr: Address, buf: &[u32]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            (&*self).write_u32(addr + 4 * i, value)?;
        }
        Ok(())
    }

    fn write_u64s(&mut self, addr: Address, buf: &[u64]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            (&*self).write_u64(addr + 8 * i, value)?;
        }
        Ok(())
    }

    fn write_addrs(&mut self, addr: Address, buf: &[Address]) -> Result<(), MemoryError> {
        for (i, value) in buf.iter().copied().enumerate() {
            (&*self).write_u32(addr + 4 * i, value.0 as u32)?;
        }
        Ok(())
    }
}
