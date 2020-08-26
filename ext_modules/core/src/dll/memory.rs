//! Implementation of `Memory` for a loaded DLL.

use super::{
    layout::{load_layout_from_dll, DllSegment},
    DllError, DllErrorCause,
};
use crate::{
    data_path::DataPathCache,
    error::Error,
    memory::{
        data_type::{FloatType, IntType},
        AddressValue, ClassifiedAddress, DataLayout, FloatValue, IntValue, Memory as MemoryTrait,
        MemoryErrorCause,
    },
};
use derive_more::Display;
use dlopen::raw::{AddressInfoObtainer, Library};
use itertools::Itertools;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    env,
    fmt::Display,
    mem,
    ops::Add,
    path::Path,
    slice,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
};
use winapi::um::{dbghelp::SymCleanup, processthreadsapi::GetCurrentProcess};

lazy_static! {
    static ref NEXT_MEMORY_ID: Mutex<usize> = Mutex::new(1);
}

/// A backup buffer that can hold the data segments of the DLL.
#[derive(Debug, Display)]
#[display(fmt = "buf[{}]", id)]
pub struct BufferSlot {
    memory_id: usize,
    id: usize,
    segments: Vec<Vec<u8>>,
}

impl BufferSlot {
    fn segment(&self, index: usize) -> Option<&[u8]> {
        self.segments.get(index).map(|seg| seg.as_slice())
    }

    fn segment_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        self.segments.get_mut(index).map(|seg| seg.as_mut_slice())
    }
}

/// The slot representing the DLL's loaded memory.
#[derive(Debug, Display)]
#[display(fmt = "base")]
pub struct BaseSlot {
    memory_id: usize,
    base_pointer: BasePointer,
    base_size: usize,
    data_segments: Vec<DllSegment>,
}

impl BaseSlot {
    /// # Safety
    /// No other pointers should write to the DLL memory while the slice is live.
    unsafe fn segment(&self, index: usize) -> Option<&[u8]> {
        let info = self.data_segments.get(index)?;
        let segment_pointer = self.base_pointer.0.wrapping_add(info.virtual_address);
        Some(slice::from_raw_parts(segment_pointer, info.virtual_size))
    }

    /// # Safety
    /// No other pointers should access the DLL memory while the slice is live.
    unsafe fn segment_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        let info = self.data_segments.get(index)?;
        let segment_pointer = self.base_pointer.0.wrapping_add(info.virtual_address);
        Some(slice::from_raw_parts_mut(
            segment_pointer,
            info.virtual_size,
        ))
    }
}

/// An id for the base slot or any backup slot.
#[derive(Debug, Display)]
pub enum Slot {
    /// Base slot, backed by DLL memory.
    Base(BaseSlot),
    /// Buffer slot, allocated by user.
    Buffer(BufferSlot),
}

impl Slot {
    fn memory_id(&self) -> usize {
        match self {
            Slot::Base(slot) => slot.memory_id,
            Slot::Buffer(slot) => slot.memory_id,
        }
    }

    unsafe fn segment(&self, index: usize) -> Option<&[u8]> {
        match self {
            Slot::Base(slot) => slot.segment(index),
            Slot::Buffer(slot) => slot.segment(index),
        }
    }

    unsafe fn segment_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        match self {
            Slot::Base(slot) => slot.segment_mut(index),
            Slot::Buffer(slot) => slot.segment_mut(index),
        }
    }
}

/// A raw address that can be stored in the DLL.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display(fmt = "{:?}", _0)]
pub struct Address(*const u8);

impl Add<usize> for Address {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.wrapping_add(rhs))
    }
}

impl From<Address> for AddressValue {
    fn from(address: Address) -> Self {
        AddressValue(address.0 as usize)
    }
}

impl From<AddressValue> for Address {
    fn from(value: AddressValue) -> Self {
        Self(value.0 as *const u8)
    }
}

/// An address in DLL memory that does not belong to a slot.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display(fmt = "static+{:010X}", _0)]
pub struct StaticAddress(usize);

/// An address that can be relocated to any slot.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
#[display(fmt = "seg[{}]+{:010X}", segment, offset)]
pub struct RelocatableAddress {
    segment: usize,
    offset: usize,
}

#[derive(Debug, Clone, Copy)]
struct BasePointer(*mut u8);

// The DLL's memory is always accessed via a Slot object (read-write) or
// a static address (read-only), so Rust's borrow rules enforce safe accesses.
unsafe impl Send for BasePointer {}
unsafe impl Sync for BasePointer {}

/// Memory management for a loaded DLL and backup slots.
///
/// Please note that working with DLLs in this way is inherently unsafe (in the Rust sense),
/// especially since `Memory` allows reading and writing to arbitrary locations in the DLL's
/// memory.
#[derive(Debug)]
pub struct Memory {
    id: usize,
    /// The loaded DLL
    library: Library,
    base_pointer: BasePointer,
    base_size: usize,
    /// Info on the segments that are included in backup slots (.data and .bss).
    data_segments: Vec<DllSegment>,
    layout: DataLayout,
    next_buffer_id: AtomicUsize,
    update_function: unsafe extern "C" fn(),
    data_path_cache: DataPathCache,
}

impl Memory {
    /// Load a DLL and extract its memory layout.
    ///
    /// # Safety
    /// Loading the same DLL multiple times is unsafe.
    ///
    /// Furthermore if the DLL is accessed from anywhere else, this will likely result in UB.
    pub unsafe fn load(
        dll_path: impl AsRef<Path> + Display,
        init_function: &str,
        update_function: &str,
    ) -> Result<(Self, Slot), Error> {
        let result: Result<(Self, Slot), DllError> = try {
            let layout = load_layout_from_dll(dll_path.as_ref())?;

            let library = Library::open(dll_path.as_ref())?;

            // When a backtrace is created, SymInitializeW is called. This causes an error
            // when AddressInfoObtainer calls the same function.
            let backtrace_enabled = match env::var("RUST_BACKTRACE").as_deref() {
                Ok("0") | Ok("") | Ok("false") | Err(_) => false,
                _ => true,
            };
            if backtrace_enabled {
                // Only do this if backtraces are enabled, since it's hacky and could break things.
                SymCleanup(GetCurrentProcess());
            }

            // dlopen API requires looking up a symbol to get the base address
            let init_function: *const () = read_symbol(&library, init_function)?;
            let addr_info = AddressInfoObtainer::new().obtain(init_function)?;
            let base_pointer = addr_info.dll_base_addr;

            // This cast is UB, but there's not really a way to avoid it when doing this kind of
            // thing with a DLL
            let base_pointer = BasePointer(base_pointer as *mut u8);

            let base_size = layout
                .segments
                .iter()
                .map(|segment| segment.virtual_address + segment.virtual_size)
                .max()
                .unwrap_or(0);

            // .data and .bss segments are the only ones that need to be copied in backup slots
            let segments: HashMap<String, DllSegment> = layout
                .segments
                .into_iter()
                .map(|segment| (segment.name.clone(), segment))
                .collect();
            let get_segment = |name: &str| {
                segments
                    .get(name)
                    .cloned()
                    .ok_or_else(|| DllErrorCause::MissingSegment {
                        name: name.to_owned(),
                    })
            };
            let data_segments = vec![get_segment(".data")?, get_segment(".bss")?];

            // Need to ensure that data segments are disjoint for aliasing restrictions
            for (segment1, segment2) in data_segments
                .iter()
                .sorted_by_key(|segment| segment.virtual_address)
                .tuple_windows()
            {
                if segment1.virtual_address + segment1.virtual_size > segment2.virtual_address {
                    Err(DllErrorCause::OverlappingSegments {
                        name1: segment1.name.clone(),
                        name2: segment2.name.clone(),
                    })?;
                }
            }

            // Call init function
            let init_function: unsafe extern "C" fn() = mem::transmute(init_function);
            init_function();

            let update_function: *const () = read_symbol(&library, update_function)?;
            let update_function: unsafe extern "C" fn() = mem::transmute(update_function);

            let mut next_memory_id = NEXT_MEMORY_ID.lock().unwrap();
            *next_memory_id = next_memory_id.checked_add(1).unwrap();
            let id = *next_memory_id;

            let memory = Self {
                id,
                library,
                base_pointer,
                base_size,
                data_segments: data_segments.clone(),
                layout: layout.data_layout,
                next_buffer_id: AtomicUsize::new(1),
                update_function,
                data_path_cache: DataPathCache::new(),
            };

            let base_slot = Slot::Base(BaseSlot {
                memory_id: memory.id,
                base_pointer,
                base_size,
                data_segments,
            });

            (memory, base_slot)
        };
        result.map_err(|error| error.context(format!("{}", dll_path)).into())
    }

    fn validate_slot(&self, slot: &Slot) -> Result<(), Error> {
        if slot.memory_id() != self.id {
            Err(MemoryErrorCause::SlotFromDifferentMemory.into())
        } else {
            Ok(())
        }
    }

    fn validate_base_slot<'a, 'b>(&'a self, slot: &'b Slot) -> Result<&'b BaseSlot, Error> {
        self.validate_slot(slot)?;
        if let Slot::Base(base_slot) = slot {
            Ok(base_slot)
        } else {
            Err(MemoryErrorCause::NonBaseSlot {
                slot: slot.to_string(),
            }
            .into())
        }
    }

    fn validate_offset<T, A: Display>(
        &self,
        address: A,
        offset: usize,
        range_size: usize,
    ) -> Result<(), Error> {
        if offset + mem::size_of::<T>() > range_size {
            Err(MemoryErrorCause::InvalidAddress {
                address: address.to_string(),
            })?
        } else if offset % mem::align_of::<T>() != 0 {
            Err(MemoryErrorCause::InvalidAddress {
                address: address.to_string(),
            })?
        } else {
            Ok(())
        }
    }

    /// Translate the static address to a pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn static_to_pointer<T>(&self, address: StaticAddress) -> Result<*const T, Error> {
        let offset = address.0;
        self.validate_offset::<T, _>(address, offset, self.base_size)?;
        Ok(self.base_pointer.0.wrapping_add(offset) as *const T)
    }

    /// Translate the relocatable address to a pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn relocatable_to_pointer<T>(
        &self,
        slot: &Slot,
        address: RelocatableAddress,
    ) -> Result<*const T, Error> {
        self.validate_slot(slot)?;
        unsafe {
            let segment =
                slot.segment(address.segment)
                    .ok_or_else(|| MemoryErrorCause::InvalidAddress {
                        address: address.to_string(),
                    })?;
            self.validate_offset::<T, _>(address, address.offset, segment.len())?;
            Ok(&segment[address.offset] as *const u8 as *const T)
        }
    }

    /// Translate the relocatable address to a mutable pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn relocatable_to_pointer_mut<T>(
        &self,
        slot: &mut Slot,
        address: RelocatableAddress,
    ) -> Result<*mut T, Error> {
        self.validate_slot(slot)?;
        unsafe {
            let segment = slot.segment_mut(address.segment).ok_or_else(|| {
                MemoryErrorCause::InvalidAddress {
                    address: address.to_string(),
                }
            })?;
            self.validate_offset::<T, _>(address, address.offset, segment.len())?;
            Ok(&mut segment[address.offset] as *mut u8 as *mut T)
        }
    }

    /// Translate the static address to a mutable pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn static_to_pointer_mut<T>(&self, address: StaticAddress) -> Result<*mut T, Error> {
        let offset = address.0;
        self.validate_offset::<T, _>(address, offset, self.base_size)?;
        Ok(self.base_pointer.0.wrapping_add(offset) as *mut T)
    }

    /// Translate an address to a pointer.
    ///
    /// # Safety
    /// This should not be used to write to memory (static or slot).
    /// This includes any functions that are called through it.
    ///
    /// The Memory must stay live while this pointer is live.
    pub unsafe fn address_to_base_pointer<T>(
        &self,
        base_slot: &Slot,
        address: &Address,
    ) -> Result<*const T, Error> {
        self.validate_base_slot(base_slot)?;
        let address = self.classify_address(address)?;
        Ok(match address {
            ClassifiedAddress::Static(address) => self.static_to_pointer(address)?,
            ClassifiedAddress::Relocatable(address) => {
                self.relocatable_to_pointer(base_slot, address)?
            }
        })
    }

    /// Translate an address to a mutable pointer.
    ///
    /// # Safety
    /// This should not be used to write to static memory (not .bss or .data).
    /// This includes any functions that are called through it.
    ///
    /// The Memory must stay live while this pointer is live.
    pub unsafe fn address_to_base_pointer_mut<T>(
        &self,
        base_slot: &mut Slot,
        address: &Address,
    ) -> Result<*mut T, Error> {
        self.validate_base_slot(base_slot)?;
        let address = self.classify_address(address)?;
        Ok(match address {
            ClassifiedAddress::Static(address) => self.static_to_pointer_mut(address)?,
            ClassifiedAddress::Relocatable(address) => {
                self.relocatable_to_pointer_mut(base_slot, address)?
            }
        })
    }

    /// Looks up the addresses for every symbol, skipping those where lookup fails.
    ///
    /// This is equivalent to calling `symbol_address` one-by-one and ignoring errors,
    /// but is faster since it skips error handling logic.
    pub fn all_symbol_address(&self) -> HashMap<&str, Address> {
        self.layout
            .globals
            .keys()
            .filter_map(|name| {
                let pointer = read_symbol_direct(&self.library, name).ok()?;
                Some((name.as_ref(), Address(pointer)))
            })
            .collect()
    }
}

impl MemoryTrait for Memory {
    type Slot = Slot;
    type Address = Address;
    type StaticAddress = StaticAddress;
    type RelocatableAddress = RelocatableAddress;

    fn read_slot_int(
        &self,
        slot: &Self::Slot,
        address: &Self::RelocatableAddress,
        int_type: IntType,
    ) -> Result<IntValue, Error> {
        unsafe {
            Ok(match int_type {
                IntType::U8 => (*self.relocatable_to_pointer::<u8>(slot, *address)?).into(),
                IntType::S8 => (*self.relocatable_to_pointer::<i8>(slot, *address)?).into(),
                IntType::U16 => (*self.relocatable_to_pointer::<u16>(slot, *address)?).into(),
                IntType::S16 => (*self.relocatable_to_pointer::<i16>(slot, *address)?).into(),
                IntType::U32 => (*self.relocatable_to_pointer::<u32>(slot, *address)?).into(),
                IntType::S32 => (*self.relocatable_to_pointer::<i32>(slot, *address)?).into(),
                IntType::U64 => (*self.relocatable_to_pointer::<u64>(slot, *address)?).into(),
                IntType::S64 => (*self.relocatable_to_pointer::<i64>(slot, *address)?).into(),
            })
        }
    }

    fn read_slot_float(
        &self,
        slot: &Self::Slot,
        address: &Self::RelocatableAddress,
        float_type: FloatType,
    ) -> Result<FloatValue, Error> {
        unsafe {
            Ok(match float_type {
                FloatType::F32 => (*self.relocatable_to_pointer::<f32>(slot, *address)?).into(),
                FloatType::F64 => (*self.relocatable_to_pointer::<f64>(slot, *address)?).into(),
            })
        }
    }

    fn read_slot_address(
        &self,
        slot: &Self::Slot,
        address: &Self::RelocatableAddress,
    ) -> Result<Self::Address, Error> {
        unsafe {
            let pointer = *self.relocatable_to_pointer::<*const u8>(slot, *address)?;
            Ok(Address(pointer))
        }
    }

    fn read_static_int(
        &self,
        address: &Self::StaticAddress,
        int_type: IntType,
    ) -> Result<IntValue, Error> {
        unsafe {
            Ok(match int_type {
                IntType::U8 => (*self.static_to_pointer::<u8>(*address)?).into(),
                IntType::S8 => (*self.static_to_pointer::<i8>(*address)?).into(),
                IntType::U16 => (*self.static_to_pointer::<u16>(*address)?).into(),
                IntType::S16 => (*self.static_to_pointer::<i16>(*address)?).into(),
                IntType::U32 => (*self.static_to_pointer::<u32>(*address)?).into(),
                IntType::S32 => (*self.static_to_pointer::<i32>(*address)?).into(),
                IntType::U64 => (*self.static_to_pointer::<u64>(*address)?).into(),
                IntType::S64 => (*self.static_to_pointer::<i64>(*address)?).into(),
            })
        }
    }

    fn read_static_float(
        &self,
        address: &Self::StaticAddress,
        float_type: FloatType,
    ) -> Result<FloatValue, Error> {
        unsafe {
            Ok(match float_type {
                FloatType::F32 => (*self.static_to_pointer::<f32>(*address)?).into(),
                FloatType::F64 => (*self.static_to_pointer::<f64>(*address)?).into(),
            })
        }
    }

    fn read_static_address(&self, address: &Self::StaticAddress) -> Result<Self::Address, Error> {
        unsafe {
            let pointer = *self.static_to_pointer::<*const u8>(*address)?;
            Ok(Address(pointer))
        }
    }

    fn write_slot_int(
        &self,
        slot: &mut Self::Slot,
        address: &Self::RelocatableAddress,
        int_type: IntType,
        value: IntValue,
    ) -> Result<(), Error> {
        unsafe {
            Ok(match int_type {
                IntType::U8 => {
                    *self.relocatable_to_pointer_mut::<u8>(slot, *address)? = value as u8
                }
                IntType::S8 => {
                    *self.relocatable_to_pointer_mut::<i8>(slot, *address)? = value as i8
                }
                IntType::U16 => {
                    *self.relocatable_to_pointer_mut::<u16>(slot, *address)? = value as u16
                }
                IntType::S16 => {
                    *self.relocatable_to_pointer_mut::<i16>(slot, *address)? = value as i16
                }
                IntType::U32 => {
                    *self.relocatable_to_pointer_mut::<u32>(slot, *address)? = value as u32
                }
                IntType::S32 => {
                    *self.relocatable_to_pointer_mut::<i32>(slot, *address)? = value as i32
                }
                IntType::U64 => {
                    *self.relocatable_to_pointer_mut::<u64>(slot, *address)? = value as u64
                }
                IntType::S64 => {
                    *self.relocatable_to_pointer_mut::<i64>(slot, *address)? = value as i64
                }
            })
        }
    }

    fn write_slot_float(
        &self,
        slot: &mut Self::Slot,
        address: &Self::RelocatableAddress,
        float_type: FloatType,
        value: FloatValue,
    ) -> Result<(), Error> {
        unsafe {
            Ok(match float_type {
                FloatType::F32 => {
                    *self.relocatable_to_pointer_mut::<f32>(slot, *address)? = value as f32
                }
                FloatType::F64 => {
                    *self.relocatable_to_pointer_mut::<f64>(slot, *address)? = value as f64
                }
            })
        }
    }

    fn write_slot_address(
        &self,
        slot: &mut Self::Slot,
        address: &Self::RelocatableAddress,
        value: &Self::Address,
    ) -> Result<(), Error> {
        unsafe { Ok(*self.relocatable_to_pointer_mut::<*const u8>(slot, *address)? = value.0) }
    }

    fn classify_address(&self, address: &Self::Address) -> Result<ClassifiedAddress<Self>, Error> {
        let offset = (address.0 as usize).wrapping_sub(self.base_pointer.0 as usize);
        if offset >= self.base_size {
            Err(MemoryErrorCause::InvalidAddress {
                address: address.to_string(),
            })?
        }

        let segment = self
            .data_segments
            .iter()
            .enumerate()
            .filter(|(_, segment)| {
                offset >= segment.virtual_address
                    && offset < segment.virtual_address + segment.virtual_size
            })
            .next();

        Ok(match segment {
            Some((i, segment)) => ClassifiedAddress::Relocatable(RelocatableAddress {
                segment: i,
                offset: offset - segment.virtual_address,
            }),
            None => ClassifiedAddress::Static(StaticAddress(offset)),
        })
    }

    fn data_layout(&self) -> &DataLayout {
        &self.layout
    }

    fn data_layout_mut(&mut self) -> &mut DataLayout {
        &mut self.layout
    }

    fn symbol_address(&self, symbol: &str) -> Result<Self::Address, Error> {
        let pointer = read_symbol(&self.library, symbol)?;
        Ok(Address(pointer))
    }

    fn data_path_cache(&self) -> &DataPathCache {
        &self.data_path_cache
    }

    fn create_backup_slot(&self) -> Result<Self::Slot, Error> {
        let id = self.next_buffer_id.fetch_add(1, Ordering::SeqCst);
        Ok(Slot::Buffer(BufferSlot {
            memory_id: self.id,
            id,
            segments: self
                .data_segments
                .iter()
                .map(|segment| vec![0; segment.virtual_size])
                .collect(),
        }))
    }

    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot) -> Result<(), Error> {
        self.validate_slot(dst)?;
        self.validate_slot(src)?;
        for i in 0..self.data_segments.len() {
            unsafe {
                let dst_segment = dst.segment_mut(i).unwrap();
                let src_segment = src.segment(i).unwrap();
                dst_segment.copy_from_slice(src_segment);
            }
        }
        Ok(())
    }

    fn advance_base_slot(&self, base_slot: &mut Self::Slot) -> Result<(), Error> {
        self.validate_base_slot(base_slot)?;
        unsafe {
            (self.update_function)();
        }
        Ok(())
    }
}

fn read_symbol<T>(library: &Library, name: &str) -> Result<*const T, DllError> {
    read_symbol_direct(library, name).map_err(|error| {
        DllErrorCause::SymbolReadError {
            name: name.to_owned(),
            source: error,
        }
        .into()
    })
}

/// Faster than read_symbol in the error path since it doesn't generate a backtrace.
fn read_symbol_direct<T>(library: &Library, name: &str) -> Result<*const T, dlopen::Error> {
    unsafe { library.symbol(name) }
}
