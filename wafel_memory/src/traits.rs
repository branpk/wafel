use core::fmt;
use std::ops::Deref;

use bytemuck::{cast_slice, cast_slice_mut};
use wafel_data_type::{Address, FloatType, FloatValue, IntType, IntValue};

use crate::MemoryError;

/// Trait for looking up a symbol's address.
///
/// A symbol is the name of a global variable or function.
pub trait SymbolLookup: fmt::Debug {
    /// Look up a symbol's address.
    ///
    /// Returns None if the symbol is undefined.
    fn symbol_address(&self, symbol: &str) -> Option<Address>;
}

impl<R, M> SymbolLookup for R
where
    R: Deref<Target = M> + fmt::Debug,
    M: SymbolLookup,
{
    fn symbol_address(&self, symbol: &str) -> Option<Address> {
        self.deref().symbol_address(symbol)
    }
}

/// Implementation of [SymbolLookup] that always returns None.
#[derive(Debug, Clone, Copy)]
pub struct EmptySymbolLookup;

impl SymbolLookup for EmptySymbolLookup {
    fn symbol_address(&self, _symbol: &str) -> Option<Address> {
        None
    }
}

/// Trait for a view of memory that allows reading values by address.
///
/// Endianness should be handled by the implementer.
pub trait MemoryRead {
    /// Read an array of u8s from the given address.
    fn read_u8s(&self, addr: Address, buf: &mut [u8]) -> Result<(), MemoryError>;

    /// Read a u8 from the given address.
    fn read_u8(&self, addr: Address) -> Result<u8, MemoryError> {
        let buf = [0];
        self.read_u8s(addr, &mut buf)?;
        Ok(buf[0])
    }

    /// Read an array of i8s from the given address.
    fn read_i8s(&self, addr: Address, buf: &mut [i8]) -> Result<(), MemoryError> {
        self.read_i8s(addr, cast_slice_mut(buf))
    }

    /// Read an i8 from the given address.
    fn read_i8(&self, addr: Address) -> Result<i8, MemoryError> {
        self.read_u8(addr).map(|n| n as i8)
    }

    /// Read an array of u16s from the given address.
    fn read_u16s(&self, addr: Address, buf: &mut [u16]) -> Result<(), MemoryError>;

    /// Read a u16 from the given address.
    fn read_u16(&self, addr: Address) -> Result<u16, MemoryError> {
        let buf = [0];
        self.read_u16s(addr, &mut buf)?;
        Ok(buf[0])
    }

    /// Read an array of i16s from the given address.
    fn read_i16s(&self, addr: Address, buf: &mut [i16]) -> Result<(), MemoryError> {
        self.read_i16s(addr, cast_slice_mut(buf))
    }

    /// Read an i16 from the given address.
    fn read_i16(&self, addr: Address) -> Result<i16, MemoryError> {
        self.read_u16(addr).map(|n| n as i16)
    }

    /// Read an array of u32s from the given address.
    fn read_u32s(&self, addr: Address, buf: &mut [u32]) -> Result<(), MemoryError>;

    /// Read a u32 from the given address.
    fn read_u32(&self, addr: Address) -> Result<u32, MemoryError> {
        let buf = [0];
        self.read_u32s(addr, &mut buf)?;
        Ok(buf[0])
    }

    /// Read an array of i32s from the given address.
    fn read_i32s(&self, addr: Address, buf: &mut [i32]) -> Result<(), MemoryError> {
        self.read_i32s(addr, cast_slice_mut(buf))
    }

    /// Read an i32 from the given address.
    fn read_i32(&self, addr: Address) -> Result<i32, MemoryError> {
        self.read_u32(addr).map(|n| n as i32)
    }

    /// Read an array of u64s from the given address.
    fn read_u64s(&self, addr: Address, buf: &mut [u64]) -> Result<(), MemoryError>;

    /// Read a u64 from the given address.
    fn read_u64(&self, addr: Address) -> Result<u64, MemoryError> {
        let buf = [0];
        self.read_u64s(addr, &mut buf)?;
        Ok(buf[0])
    }

    /// Read an array of i64s from the given address.
    fn read_i64s(&self, addr: Address, buf: &mut [i64]) -> Result<(), MemoryError> {
        self.read_i64s(addr, cast_slice_mut(buf))
    }

    /// Read an i64 from the given address.
    fn read_i64(&self, addr: Address) -> Result<i64, MemoryError> {
        self.read_u64(addr).map(|n| n as i64)
    }

    /// Read an array of f32s from the given address.
    fn read_f32s(&self, addr: Address, buf: &mut [f32]) -> Result<(), MemoryError> {
        self.read_u32s(addr, cast_slice_mut(buf))
    }

    /// Read an f32 from the given address.
    fn read_f32(&self, addr: Address) -> Result<f32, MemoryError> {
        let buf = [0.0];
        self.read_f32s(addr, &mut buf)?;
        Ok(buf[0])
    }

    /// Read an array of f64s from the given address.
    fn read_f64s(&self, addr: Address, buf: &mut [f64]) -> Result<(), MemoryError> {
        self.read_u64s(addr, cast_slice_mut(buf))
    }

    /// Read an f64 from the given address.
    fn read_f64(&self, addr: Address) -> Result<f64, MemoryError> {
        let buf = [0.0];
        self.read_f64s(addr, &mut buf)?;
        Ok(buf[0])
    }

    /// Read an array of pointers from the given address.
    fn read_addrs(&self, addr: Address, buf: &mut [Address]) -> Result<(), MemoryError>;

    /// Read a pointer from the given address.
    fn read_addr(&self, addr: Address) -> Result<Address, MemoryError> {
        let buf = [Address::NULL];
        self.read_addrs(addr, &mut buf)?;
        Ok(buf[0])
    }

    /// Read an int from the given address.
    ///
    /// The int's size and signedness is given by `int_type`.
    fn read_int(&self, addr: Address, int_type: IntType) -> Result<IntValue, MemoryError> {
        match int_type {
            IntType::U8 => self.read_u8(addr).map(IntValue::from),
            IntType::S8 => self.read_i8(addr).map(IntValue::from),
            IntType::U16 => self.read_u16(addr).map(IntValue::from),
            IntType::S16 => self.read_i16(addr).map(IntValue::from),
            IntType::U32 => self.read_u32(addr).map(IntValue::from),
            IntType::S32 => self.read_i32(addr).map(IntValue::from),
            IntType::U64 => self.read_u64(addr).map(IntValue::from),
            IntType::S64 => self.read_i64(addr).map(IntValue::from),
        }
    }

    /// Read a float from the given address.
    ///
    /// The float's size and signedness is given by `float_type`.
    fn read_float(&self, addr: Address, float_type: FloatType) -> Result<FloatValue, MemoryError> {
        match float_type {
            FloatType::F32 => self.read_f32(addr).map(FloatValue::from),
            FloatType::F64 => self.read_f64(addr).map(FloatValue::from),
        }
    }

    /// Read a null terminated C string from the given address.
    fn read_string(&self, addr: Address) -> Result<Vec<u8>, MemoryError> {
        let mut bytes = Vec::new();
        let mut current = addr;
        loop {
            let byte = self.read_u8(current)?;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            current += 1;
        }
        Ok(bytes)
    }

    /// Return the int type corresponding to a pointer (either U32 or U64).
    fn pointer_int_type(&self) -> IntType;
}

/// Trait for a view of memory that allows writing values by address.
///
/// Endianness should be handled by the implementer.
pub trait MemoryWrite {
    /// Write an array of u8s to the given address.
    fn write_u8s(&mut self, addr: Address, buf: &[u8]) -> Result<(), MemoryError>;

    /// Write a u8 to the given address.
    fn write_u8(&mut self, addr: Address, value: u8) -> Result<(), MemoryError> {
        self.write_u8s(addr, &[value])
    }

    /// Write an array of i8s to the given address.
    fn write_i8s(&mut self, addr: Address, buf: &[i8]) -> Result<(), MemoryError> {
        self.write_u8s(addr, cast_slice(buf))
    }

    /// Write an i8 to the given address.
    fn write_i8(&mut self, addr: Address, value: i8) -> Result<(), MemoryError> {
        self.write_i8s(addr, &[value])
    }

    /// Write an array of u16s to the given address.
    fn write_u16s(&mut self, addr: Address, buf: &[u16]) -> Result<(), MemoryError>;

    /// Write a u16 to the given address.
    fn write_u16(&mut self, addr: Address, value: u16) -> Result<(), MemoryError> {
        self.write_u16s(addr, &[value])
    }

    /// Write an array of i16s to the given address.
    fn write_i16s(&mut self, addr: Address, buf: &[i16]) -> Result<(), MemoryError> {
        self.write_u16s(addr, cast_slice(buf))
    }

    /// Write an i16 to the given address.
    fn write_i16(&mut self, addr: Address, value: i16) -> Result<(), MemoryError> {
        self.write_i16s(addr, &[value])
    }

    /// Write an array of u32s to the given address.
    fn write_u32s(&mut self, addr: Address, buf: &[u32]) -> Result<(), MemoryError>;

    /// Write a u32 to the given address.
    fn write_u32(&mut self, addr: Address, value: u32) -> Result<(), MemoryError> {
        self.write_u32s(addr, &[value])
    }

    /// Write an array of i32s to the given address.
    fn write_i32s(&mut self, addr: Address, buf: &[i32]) -> Result<(), MemoryError> {
        self.write_u32s(addr, cast_slice(buf))
    }

    /// Write an i32 to the given address.
    fn write_i32(&mut self, addr: Address, value: i32) -> Result<(), MemoryError> {
        self.write_i32s(addr, &[value])
    }

    /// Write an array of u64s to the given address.
    fn write_u64s(&mut self, addr: Address, buf: &[u64]) -> Result<(), MemoryError>;

    /// Write a u64 to the given address.
    fn write_u64(&mut self, addr: Address, value: u64) -> Result<(), MemoryError> {
        self.write_u64s(addr, &[value])
    }

    /// Write an array of i64s to the given address.
    fn write_i64s(&mut self, addr: Address, buf: &[i64]) -> Result<(), MemoryError> {
        self.write_u64s(addr, cast_slice(buf))
    }

    /// Write an i64 to the given address.
    fn write_i64(&mut self, addr: Address, value: i64) -> Result<(), MemoryError> {
        self.write_i64s(addr, &[value])
    }

    /// Write an array of f32s to the given address.
    fn write_f32s(&mut self, addr: Address, buf: &[f32]) -> Result<(), MemoryError> {
        self.write_u32s(addr, cast_slice(buf))
    }

    /// Write an f32 to the given address.
    fn write_f32(&mut self, addr: Address, value: f32) -> Result<(), MemoryError> {
        self.write_f32s(addr, &[value])
    }

    /// Write an array of f64s to the given address.
    fn write_f64s(&mut self, addr: Address, buf: &[f64]) -> Result<(), MemoryError> {
        self.write_u64s(addr, cast_slice(buf))
    }

    /// Write an f64 to the given address.
    fn write_f64(&mut self, addr: Address, value: f64) -> Result<(), MemoryError> {
        self.write_f64s(addr, &[value])
    }

    /// Write an array of pointer values at the given address.
    ///
    /// The pointer values may be invalid or zero.
    fn write_addrs(&mut self, addr: Address, buf: &[Address]) -> Result<(), MemoryError>;

    /// Write a pointer value at the given address.
    ///
    /// The pointer value may be invalid or zero.
    fn write_addr(&mut self, addr: Address, value: Address) -> Result<(), MemoryError> {
        self.write_addrs(addr, &[value])
    }

    /// Write an int at the given address.
    ///
    /// The int's size and signedness is given by `int_type`.
    fn write_int(
        &mut self,
        addr: Address,
        int_type: IntType,
        value: IntValue,
    ) -> Result<(), MemoryError> {
        match int_type {
            IntType::U8 => self.write_u8(addr, value as u8),
            IntType::S8 => self.write_i8(addr, value as i8),
            IntType::U16 => self.write_u16(addr, value as u16),
            IntType::S16 => self.write_i16(addr, value as i16),
            IntType::U32 => self.write_u32(addr, value as u32),
            IntType::S32 => self.write_i32(addr, value as i32),
            IntType::U64 => self.write_u64(addr, value as u64),
            IntType::S64 => self.write_i64(addr, value as i64),
        }
    }

    /// Write a float at the given address.
    ///
    /// The float's size and signedness is given by `float_type`.
    fn write_float(
        &mut self,
        addr: Address,
        float_type: FloatType,
        value: FloatValue,
    ) -> Result<(), MemoryError> {
        match float_type {
            FloatType::F32 => self.write_f32(addr, value as f32),
            FloatType::F64 => self.write_f64(addr, value),
        }
    }
}

/// A trait that allows accessing game memory and saving/loading states.
///
/// The memory is divided into static and non-static memory.
/// Static memory is considered immutable, e.g. the .code and .rodata sections, while
/// non-static memory is mutable and includes the .data and .bss sections.
///
/// A "slot" is a buffer that can hold a copy of non-static memory.
/// Each [GameMemory] comes with a base slot, which represents the game's actual loaded memory.
/// Other backup slots can be created, and data can be copied to/from the base slot.
pub trait GameMemory {
    /// A buffer that can hold a copy of non-static memory.
    type Slot;

    /// A read-only view of shared static memory.
    ///
    /// Attempting to read a non-static address using this will fail.
    type StaticView<'a>: MemoryRead
    where
        Self: 'a;

    /// A read-only view of both static and non-static memory, backed by a
    /// particular slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    type SlotView<'a>: MemoryRead
    where
        Self: 'a;

    /// A read-write view of both static and non-static memory, backed by a
    /// particular slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    type SlotViewMut<'a>: MemoryRead + MemoryWrite
    where
        Self: 'a;

    /// Return a read-only view of shared static memory.
    ///
    /// Attempting to read a non-static address using this will fail.
    fn static_view(&self) -> Self::StaticView<'_>;

    /// Return a read-only view of both static and non-static memory, backed by the given
    /// slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    fn with_slot<'a>(&'a self, slot: &'a Self::Slot) -> Self::SlotView<'a>;

    /// Return a read-write view of both static and non-static memory, backed by the given
    /// slot.
    ///
    /// Static addresses are looked up in shared static memory, while non-static
    /// addresses are looked up in the slot's buffer.
    fn with_slot_mut<'a>(&'a self, slot: &'a mut Self::Slot) -> Self::SlotViewMut<'a>;

    /// Allocate a new backup slot.
    ///
    /// Note that a slot can be large, so care should be taken to avoid allocating
    /// too many. An SM64 slot is ~2 MB.
    fn create_backup_slot(&self) -> Self::Slot;

    /// Copy data from one slot to another.
    ///
    /// # Panics
    ///
    /// Panics if either slot is not owned by this [GameMemory] instance.
    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot);

    /// Advance the base slot by one frame.
    ///
    /// # Panics
    ///
    /// Panics if the slot is not the base slot, i.e. was created using
    /// [create_backup_slot](GameMemory::create_backup_slot).
    /// To advance a backup slot, you should first copy it to the base slot,
    /// then advance the base slot, then copy it back.
    fn advance_base_slot(&self, base_slot: &mut Self::Slot);
}

impl<R, M> GameMemory for R
where
    R: Deref<Target = M>,
    M: GameMemory + 'static,
{
    type Slot = M::Slot;

    type StaticView<'a>
    = M::StaticView<'a> where R:'a;

    type SlotView<'a>
    = M::SlotView<'a> where R: 'a;

    type SlotViewMut<'a>
    = M::SlotViewMut<'a> where R: 'a;

    fn static_view(&self) -> Self::StaticView<'_> {
        self.deref().static_view()
    }

    fn with_slot<'a>(&'a self, slot: &'a Self::Slot) -> Self::SlotView<'a> {
        self.deref().with_slot(slot)
    }

    fn with_slot_mut<'a>(&'a self, slot: &'a mut Self::Slot) -> Self::SlotViewMut<'a> {
        self.deref().with_slot_mut(slot)
    }

    fn create_backup_slot(&self) -> Self::Slot {
        self.deref().create_backup_slot()
    }

    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot) {
        self.deref().copy_slot(dst, src)
    }

    fn advance_base_slot(&self, base_slot: &mut Self::Slot) {
        self.deref().advance_base_slot(base_slot)
    }
}
