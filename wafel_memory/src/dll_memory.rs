use std::{
    mem, ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    usize,
};

use dlopen::raw::{AddressInfoObtainer, Library};
use once_cell::sync::OnceCell;
use wafel_data_type::Address;
use wafel_layout::{read_dll_segments, DllSegment};

use crate::{
    dll_slot_impl::{BasePointer, BaseSlot, BufferSlot, SlotImpl},
    error::DllLoadError,
    unique_dll::UniqueLibrary,
    GameMemory,
    MemoryError::{self, *},
    MemoryReadPrimitive, MemoryWritePrimitive, SymbolLookup,
};

/// A slot for [DllGameMemory].
///
/// See the documentation for [GameMemory].
#[derive(Debug)]
pub struct DllSlot(SlotImpl);

impl DllSlot {
    /// Return true if the slot is the base slot for its [DllGameMemory].
    pub fn is_base_slot(&self) -> bool {
        matches!(self.0, SlotImpl::Base(_))
    }
}

/// An address that has been classified as either static or relocatable.
#[derive(Debug, Clone, Copy)]
enum ClassifiedAddress {
    /// A static address that lies outside of any slot.
    Static { offset: usize },
    /// An address that can be relocated to a specific slot.
    Relocatable { segment: usize, offset: usize },
    /// A null or invalid address.
    Invalid,
}

#[allow(clippy::mutex_atomic)]
fn memory_id_mutex() -> &'static Mutex<usize> {
    static NEXT_MEMORY_ID: OnceCell<Mutex<usize>> = OnceCell::new();
    NEXT_MEMORY_ID.get_or_init(|| Mutex::new(1))
}

fn next_memory_id() -> usize {
    let mut next_memory_id = memory_id_mutex().lock().unwrap();
    let id = *next_memory_id;
    *next_memory_id = next_memory_id.checked_add(1).unwrap();
    id
}

/// Memory management for a loaded DLL and backup slots.
#[derive(Debug)]
pub struct DllGameMemory {
    id: usize,
    /// The loaded DLL
    library: UniqueLibrary,
    base_pointer: BasePointer,
    base_size: usize,
    /// Info on the segments that are included in backup slots (.data and .bss).
    data_segments: Vec<DllSegment>,
    next_buffer_id: AtomicUsize,
    update_function: unsafe extern "C" fn(),
}

impl DllGameMemory {
    /// Load a DLL and return a [DllGameMemory] and its base slot.
    ///
    /// `init_function_name` will be called once to initialize the game, and
    /// `update_function_name` will be called when
    /// [advance_base_slot](GameMemory::advance_base_slot) is called.
    /// Both should take no arguments.
    ///
    /// # Safety
    ///
    /// This method is inherently unsafe:
    /// - If the DLL image is modified (either on disk before load or in memory) from anywhere
    ///   except this [DllGameMemory], this is UB.
    /// - The init and update functions can run arbitrary code in the DLL.
    pub unsafe fn load(
        dll_path: &str,
        init_function_name: &str,
        update_function_name: &str,
    ) -> Result<(Self, DllSlot), DllLoadError> {
        let all_segments = read_dll_segments(dll_path)?;
        let data_segments = dll_data_segments(&all_segments)?;

        let base_size = all_segments
            .iter()
            .map(|segment| segment.virtual_address + segment.virtual_size)
            .max()
            .unwrap_or(0);

        let library = UniqueLibrary::open(dll_path)?;

        let init_function: unsafe extern "C" fn() = library
            .symbol(init_function_name)
            .map_err(|_| DllLoadError::UndefinedSymbol(init_function_name.to_string()))?;
        let update_function: unsafe extern "C" fn() = library
            .symbol(update_function_name)
            .map_err(|_| DllLoadError::UndefinedSymbol(update_function_name.to_string()))?;

        let base_pointer = dll_base_pointer(init_function as *const ())?;

        // Call init function
        init_function();

        let memory = Self {
            id: next_memory_id(),
            library,
            base_pointer,
            base_size,
            data_segments: data_segments.clone(),
            next_buffer_id: AtomicUsize::new(1),
            update_function,
        };

        let base_slot = DllSlot(SlotImpl::Base(BaseSlot::new(
            memory.id,
            base_pointer,
            base_size,
            data_segments,
        )));

        Ok((memory, base_slot))
    }

    fn validate_slot(&self, slot: &DllSlot) {
        assert_eq!(
            slot.0.memory_id(),
            self.id,
            "slot is not owned by this dll memory"
        )
    }

    fn validate_base_slot(&self, slot: &DllSlot) {
        self.validate_slot(slot);
        assert!(
            slot.is_base_slot(),
            "operation requires the base slot, but a buffer slot was used"
        );
    }

    fn validate_offset<T>(&self, offset: usize, range_size: usize) -> Result<(), MemoryError> {
        if offset + mem::size_of::<T>() > range_size || offset % mem::align_of::<T>() != 0 {
            Err(InvalidAddress)
        } else {
            Ok(())
        }
    }

    fn classify_address(&self, address: Address) -> ClassifiedAddress {
        let offset = address.0;
        if offset >= self.base_size {
            return ClassifiedAddress::Invalid;
        }

        let segment = self.data_segments.iter().enumerate().find(|(_, segment)| {
            offset >= segment.virtual_address
                && offset < segment.virtual_address + segment.virtual_size
        });

        match segment {
            Some((i, segment)) => ClassifiedAddress::Relocatable {
                segment: i,
                offset: offset - segment.virtual_address,
            },
            None => ClassifiedAddress::Static { offset },
        }
    }

    fn unchecked_pointer_to_address<T>(&self, pointer: *const T) -> Address {
        if pointer.is_null() {
            Address(0)
        } else {
            Address((pointer as usize).wrapping_sub(self.base_pointer.0 as usize))
        }
    }

    fn unchecked_address_to_pointer<T>(&self, address: Address) -> *const T {
        if address.is_null() {
            ptr::null()
        } else {
            (self.base_pointer.0 as usize).wrapping_add(address.0) as *const T
        }
    }

    /// Translate the static address to a pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn static_to_pointer<T>(&self, offset: usize) -> Result<*const T, MemoryError> {
        self.validate_offset::<T>(offset, self.base_size)?;
        Ok((self.base_pointer.0 as usize).wrapping_add(offset) as *const T)
    }

    /// Translate the relocatable address to a pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn relocatable_to_pointer<T>(
        &self,
        slot: &DllSlot,
        segment: usize,
        offset: usize,
    ) -> Result<*const T, MemoryError> {
        self.validate_slot(slot);
        unsafe {
            let segment = slot.0.segment(segment).ok_or(InvalidAddress)?;
            self.validate_offset::<T>(offset, segment.len())?;
            Ok(&segment[offset] as *const u8 as *const T)
        }
    }

    /// Translate the relocatable address to a mutable pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn relocatable_to_pointer_mut<T>(
        &self,
        slot: &mut DllSlot,
        segment: usize,
        offset: usize,
    ) -> Result<*mut T, MemoryError> {
        self.validate_slot(slot);
        unsafe {
            let segment = slot.0.segment_mut(segment).ok_or(InvalidAddress)?;
            self.validate_offset::<T>(offset, segment.len())?;
            Ok(&mut segment[offset] as *mut u8 as *mut T)
        }
    }

    /// Translate the address to a pointer, returning an error if it isn't a static address.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn address_to_static_pointer<T>(&self, address: Address) -> Result<*const T, MemoryError> {
        match self.classify_address(address) {
            ClassifiedAddress::Static { offset } => self.static_to_pointer(offset),
            ClassifiedAddress::Relocatable { .. } => Err(NonStaticAddressInStaticView),
            ClassifiedAddress::Invalid => Err(InvalidAddress),
        }
    }

    /// Translate the address to a pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn address_to_pointer<T>(
        &self,
        slot: &DllSlot,
        address: Address,
    ) -> Result<*const T, MemoryError> {
        match self.classify_address(address) {
            ClassifiedAddress::Static { offset } => self.static_to_pointer(offset),
            ClassifiedAddress::Relocatable { segment, offset } => {
                self.relocatable_to_pointer(slot, segment, offset)
            }
            ClassifiedAddress::Invalid => Err(InvalidAddress),
        }
    }

    /// Translate the address to a mutable pointer.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn address_to_pointer_mut<T>(
        &self,
        slot: &mut DllSlot,
        address: Address,
    ) -> Result<*mut T, MemoryError> {
        match self.classify_address(address) {
            ClassifiedAddress::Static { .. } => Err(WriteToStaticAddress),
            ClassifiedAddress::Relocatable { segment, offset } => {
                self.relocatable_to_pointer_mut(slot, segment, offset)
            }
            ClassifiedAddress::Invalid => Err(InvalidAddress),
        }
    }
}

impl SymbolLookup for DllGameMemory {
    fn symbol_address(&self, symbol: &str) -> Option<Address> {
        read_symbol(&self.library, symbol)
            .map(|pointer: *const u8| self.unchecked_pointer_to_address(pointer))
    }
}

impl GameMemory for DllGameMemory {
    type Slot = DllSlot;

    type StaticView<'a> = DllStaticMemoryView<'a>;
    type SlotView<'a> = DllSlotMemoryView<'a>;
    type SlotViewMut<'a> = DllSlotMemoryViewMut<'a>;

    fn static_view(&self) -> Self::StaticView<'_> {
        DllStaticMemoryView { memory: self }
    }

    fn with_slot<'a>(&'a self, slot: &'a Self::Slot) -> Self::SlotView<'a> {
        DllSlotMemoryView { memory: self, slot }
    }

    fn with_slot_mut<'a>(&'a self, slot: &'a mut Self::Slot) -> Self::SlotViewMut<'a> {
        DllSlotMemoryViewMut { memory: self, slot }
    }

    fn create_backup_slot(&self) -> Self::Slot {
        let id = self.next_buffer_id.fetch_add(1, Ordering::SeqCst);
        DllSlot(SlotImpl::Buffer(BufferSlot::new(
            self.id,
            id,
            self.data_segments
                .iter()
                .map(|segment| vec![0; segment.virtual_size])
                .collect(),
        )))
    }

    fn copy_slot(&self, dst: &mut Self::Slot, src: &Self::Slot) {
        self.validate_slot(dst);
        self.validate_slot(src);
        for i in 0..self.data_segments.len() {
            unsafe {
                let dst_segment = dst.0.segment_mut(i).unwrap();
                let src_segment = src.0.segment(i).unwrap();
                dst_segment.copy_from_slice(src_segment);
            }
        }
    }

    fn advance_base_slot(&self, base_slot: &mut Self::Slot) {
        self.validate_base_slot(base_slot);
        unsafe {
            (self.update_function)();
        }
    }
}

fn dll_data_segments(all_segments: &[DllSegment]) -> Result<Vec<DllSegment>, DllLoadError> {
    // .data and .bss segments are the only ones that need to be copied in backup slots
    let mut data_segments: Vec<DllSegment> = all_segments
        .iter()
        .filter(|&segment| [".data", ".bss", "__DATA"].contains(&segment.name.as_str()))
        .cloned()
        .collect();

    if data_segments.is_empty() {
        return Err(DllLoadError::MissingDataSegments);
    }

    // Need to ensure that data segments are disjoint for aliasing restrictions
    data_segments.sort_by_key(|segment| segment.virtual_address);
    for i in 0..data_segments.len().saturating_sub(1) {
        let segment1 = &data_segments[i];
        let segment2 = &data_segments[i + 1];
        assert!(
            segment1.virtual_address + segment1.virtual_size <= segment2.virtual_address,
            "overlapping dll segments"
        );
    }

    Ok(data_segments)
}

unsafe fn dll_base_pointer(
    arbitrary_symbol_pointer: *const (),
) -> Result<BasePointer, dlopen::Error> {
    #[cfg(windows)]
    {
        use winapi::um::{dbghelp::SymCleanup, processthreadsapi::GetCurrentProcess};

        // When a backtrace is created, SymInitializeW is called. This causes an error
        // when AddressInfoObtainer calls the same function.
        // https://github.com/szymonwieloch/rust-dlopen/issues/37
        SymCleanup(GetCurrentProcess());
    }

    // dlopen API requires looking up a symbol to get the base address
    let addr_info = AddressInfoObtainer::new().obtain(arbitrary_symbol_pointer)?;

    // This cast is UB, but there's not really a way to avoid it when doing this kind of
    // thing with a DLL
    Ok(BasePointer(addr_info.dll_base_addr as *mut u8))
}

fn read_symbol<T>(library: &Library, name: &str) -> Option<*const T> {
    unsafe { library.symbol(name) }.ok()
}

/// A read-only view of shared static memory.
///
/// See [GameMemory::static_view].
#[derive(Debug)]
pub struct DllStaticMemoryView<'a> {
    memory: &'a DllGameMemory,
}

impl MemoryReadPrimitive for DllStaticMemoryView<'_> {
    unsafe fn read_primitive<T: Copy>(&self, address: Address) -> Result<T, MemoryError> {
        self.memory
            .address_to_static_pointer::<T>(address)
            .map(|p| *p)
    }

    fn read_address(&self, address: Address) -> Result<Address, MemoryError> {
        let pointer = unsafe { self.read_primitive::<*const ()>(address)? };
        Ok(self.memory.unchecked_pointer_to_address(pointer))
    }
}

/// A read-only view of both static and non-static memory, backed by a
/// particular slot.
///
/// See [GameMemory::with_slot].
#[derive(Debug)]
pub struct DllSlotMemoryView<'a> {
    memory: &'a DllGameMemory,
    slot: &'a DllSlot,
}

impl MemoryReadPrimitive for DllSlotMemoryView<'_> {
    unsafe fn read_primitive<T: Copy>(&self, address: Address) -> Result<T, MemoryError> {
        self.memory
            .address_to_pointer::<T>(self.slot, address)
            .map(|p| *p)
    }

    fn read_address(&self, address: Address) -> Result<Address, MemoryError> {
        let pointer = unsafe { self.read_primitive::<*const ()>(address)? };
        Ok(self.memory.unchecked_pointer_to_address(pointer))
    }
}

/// A read-write view of both static and non-static memory, backed by a
/// particular slot.
///
/// See [GameMemory::with_slot_mut].
#[derive(Debug)]
pub struct DllSlotMemoryViewMut<'a> {
    memory: &'a DllGameMemory,
    slot: &'a mut DllSlot,
}

impl MemoryReadPrimitive for DllSlotMemoryViewMut<'_> {
    unsafe fn read_primitive<T: Copy>(&self, address: Address) -> Result<T, MemoryError> {
        self.memory
            .address_to_pointer::<T>(self.slot, address)
            .map(|p| *p)
    }

    fn read_address(&self, address: Address) -> Result<Address, MemoryError> {
        let pointer = unsafe { self.read_primitive::<*const ()>(address)? };
        Ok(self.memory.unchecked_pointer_to_address(pointer))
    }
}

impl MemoryWritePrimitive for DllSlotMemoryViewMut<'_> {
    unsafe fn write_primitive<T: Copy>(
        &mut self,
        address: Address,
        value: T,
    ) -> Result<(), MemoryError> {
        let pointer = self.memory.address_to_pointer_mut(self.slot, address)?;
        *pointer = value;
        Ok(())
    }

    fn write_address(&mut self, address: Address, value: Address) -> Result<(), MemoryError> {
        let value_pointer: *const () = self.memory.unchecked_address_to_pointer(value);
        unsafe {
            self.write_primitive(address, value_pointer)?;
        }
        Ok(())
    }
}
