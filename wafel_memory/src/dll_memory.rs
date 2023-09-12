use std::{
    mem, ptr, slice,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    usize,
};

use dlopen::raw::{AddressInfoObtainer, Library};
use once_cell::sync::OnceCell;
use wafel_data_type::{Address, IntType};
use wafel_layout::{append_dll_extension, dll_data_segments, read_dll_segments, DllSegment};

use crate::{
    dll_slot_impl::{BasePointer, BaseSlot, BufferSlot, SlotImpl},
    error::MemoryInitError,
    unique_dll::UniqueLibrary,
    GameMemory,
    MemoryError::{self, *},
    MemoryRead, MemoryWrite, SymbolLookup,
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
    ) -> Result<(Self, DllSlot), MemoryInitError> {
        let dll_path = append_dll_extension(dll_path);

        let all_segments = read_dll_segments(&dll_path)?;
        let data_segments = dll_data_segments(&all_segments)?;

        let base_size = all_segments
            .iter()
            .map(|segment| (segment.virtual_address + segment.virtual_size) as usize)
            .max()
            .unwrap_or(0);

        let library = UniqueLibrary::open(&dll_path)?;

        let init_function: unsafe extern "C" fn() = library
            .symbol(init_function_name)
            .map_err(|_| MemoryInitError::UndefinedSymbol(init_function_name.to_string()))?;
        let update_function: unsafe extern "C" fn() = library
            .symbol(update_function_name)
            .map_err(|_| MemoryInitError::UndefinedSymbol(update_function_name.to_string()))?;

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

    /// Return the base address of the DLL instance in memory.
    pub fn base_address(&self) -> usize {
        self.base_pointer.0 as usize
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

    fn classify_address(&self, address: Address) -> ClassifiedAddress {
        let offset = address.0;
        if offset >= self.base_size {
            return ClassifiedAddress::Invalid;
        }

        let segment = self.data_segments.iter().enumerate().find(|(_, segment)| {
            offset >= segment.virtual_address as usize
                && offset < (segment.virtual_address + segment.virtual_size) as usize
        });

        match segment {
            Some((i, segment)) => ClassifiedAddress::Relocatable {
                segment: i,
                offset: offset - segment.virtual_address as usize,
            },
            None => ClassifiedAddress::Static { offset },
        }
    }

    /// Translate the pointer in the dll to a relocatable address.
    ///
    /// This does not perform any alignment or bounds checking, but those will be done when the
    /// address is accessed.
    fn unchecked_pointer_to_address<T>(&self, pointer: *const T) -> Address {
        if pointer.is_null() {
            Address::NULL
        } else {
            let offset = (pointer as usize).wrapping_sub(self.base_pointer.0 as usize);
            Address(offset)
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
    fn static_to_pointer<T>(&self, offset: usize, len: usize) -> Result<*const T, MemoryError> {
        self.validate_offset::<T>(offset, len, self.base_size)?;
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
        len: usize,
    ) -> Result<*const T, MemoryError> {
        self.validate_slot(slot);
        unsafe {
            let segment = slot.0.segment(segment).ok_or(InvalidAddress)?;
            self.validate_offset::<T>(offset, len, segment.len())?;
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
        len: usize,
    ) -> Result<*mut T, MemoryError> {
        self.validate_slot(slot);
        unsafe {
            let segment = slot.0.segment_mut(segment).ok_or(InvalidAddress)?;
            self.validate_offset::<T>(offset, len, segment.len())?;
            Ok(&mut segment[offset] as *mut u8 as *mut T)
        }
    }

    /// Translate the address to a pointer, returning an error if it isn't a static address.
    ///
    /// Performs validation and bounds checking, so the result should be a valid pointer.
    /// Dereferencing should be safe provided junk data is acceptable in T.
    fn address_to_static_pointer<T>(
        &self,
        address: Address,
        len: usize,
    ) -> Result<*const T, MemoryError> {
        match self.classify_address(address) {
            ClassifiedAddress::Static { offset } => self.static_to_pointer(offset, len),
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
        len: usize,
    ) -> Result<*const T, MemoryError> {
        match self.classify_address(address) {
            ClassifiedAddress::Static { offset } => self.static_to_pointer(offset, len),
            ClassifiedAddress::Relocatable { segment, offset } => {
                self.relocatable_to_pointer(slot, segment, offset, len)
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
        len: usize,
    ) -> Result<*mut T, MemoryError> {
        match self.classify_address(address) {
            ClassifiedAddress::Static { .. } => Err(WriteToStaticAddress),
            ClassifiedAddress::Relocatable { segment, offset } => {
                self.relocatable_to_pointer_mut(slot, segment, offset, len)
            }
            ClassifiedAddress::Invalid => Err(InvalidAddress),
        }
    }

    /// Looks up a symbol for the underlying DLL.
    ///
    /// # Safety
    ///
    /// See [dlopen::Library::symbol]. Also, if the symbol is used to modify the base slot's
    /// memory (e.g. calling a function), base_slot should not be used until this is complete.
    pub unsafe fn symbol_pointer<T>(
        &self,
        base_slot: &mut DllSlot,
        name: &str,
    ) -> Result<T, MemoryError> {
        self.validate_base_slot(base_slot);
        let pointer: T = self
            .library
            .symbol(name)
            .map_err(|_| UndefinedSymbol(name.to_string()))?;
        Ok(pointer)
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
                .map(|segment| vec![0; segment.virtual_size as usize])
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

pub(crate) unsafe fn dll_base_pointer(
    arbitrary_symbol_pointer: *const (),
) -> Result<BasePointer, dlopen::Error> {
    #[cfg(target_os = "windows")]
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

impl MemoryRead for DllStaticMemoryView<'_> {
    fn read_u8s(&self, addr: Address, buf: &mut [u8]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_static_pointer::<u8>(addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u16s(&self, addr: Address, buf: &mut [u16]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_static_pointer::<u16>(addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u32s(&self, addr: Address, buf: &mut [u32]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_static_pointer::<u32>(addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u64s(&self, addr: Address, buf: &mut [u64]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_static_pointer::<u64>(addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_addrs(&self, addr: Address, buf: &mut [Address]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_static_pointer::<*const ()>(addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        for (dst, src) in buf.iter_mut().zip(data.iter().copied()) {
            *dst = self.memory.unchecked_pointer_to_address(src);
        }
        Ok(())
    }

    fn pointer_int_type(&self) -> IntType {
        IntType::u_ptr_native()
    }
}

/// A read-only view of both static and non-static memory, backed by a
/// particular slot.
///
/// See [GameMemory::with_slot].
#[derive(Debug, Clone, Copy)]
pub struct DllSlotMemoryView<'a> {
    memory: &'a DllGameMemory,
    slot: &'a DllSlot,
}

impl MemoryRead for DllSlotMemoryView<'_> {
    fn read_u8s(&self, addr: Address, buf: &mut [u8]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u8>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u16s(&self, addr: Address, buf: &mut [u16]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u16>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u32s(&self, addr: Address, buf: &mut [u32]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u32>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u64s(&self, addr: Address, buf: &mut [u64]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u64>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_addrs(&self, addr: Address, buf: &mut [Address]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<*const ()>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        for (dst, src) in buf.iter_mut().zip(data.iter().copied()) {
            *dst = self.memory.unchecked_pointer_to_address(src);
        }
        Ok(())
    }

    fn pointer_int_type(&self) -> IntType {
        IntType::u_ptr_native()
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

impl MemoryRead for DllSlotMemoryViewMut<'_> {
    fn read_u8s(&self, addr: Address, buf: &mut [u8]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u8>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u16s(&self, addr: Address, buf: &mut [u16]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u16>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u32s(&self, addr: Address, buf: &mut [u32]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u32>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_u64s(&self, addr: Address, buf: &mut [u64]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<u64>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        buf.copy_from_slice(data);
        Ok(())
    }

    fn read_addrs(&self, addr: Address, buf: &mut [Address]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer::<*const ()>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts(ptr, buf.len()) };
        for (dst, src) in buf.iter_mut().zip(data.iter().copied()) {
            *dst = self.memory.unchecked_pointer_to_address(src);
        }
        Ok(())
    }

    fn pointer_int_type(&self) -> IntType {
        IntType::u_ptr_native()
    }
}

impl MemoryWrite for DllSlotMemoryViewMut<'_> {
    fn write_u8s(&mut self, addr: Address, buf: &[u8]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer_mut::<u8>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts_mut(ptr, buf.len()) };
        data.copy_from_slice(buf);
        Ok(())
    }

    fn write_u16s(&mut self, addr: Address, buf: &[u16]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer_mut::<u16>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts_mut(ptr, buf.len()) };
        data.copy_from_slice(buf);
        Ok(())
    }

    fn write_u32s(&mut self, addr: Address, buf: &[u32]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer_mut::<u32>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts_mut(ptr, buf.len()) };
        data.copy_from_slice(buf);
        Ok(())
    }

    fn write_u64s(&mut self, addr: Address, buf: &[u64]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer_mut::<u64>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts_mut(ptr, buf.len()) };
        data.copy_from_slice(buf);
        Ok(())
    }

    fn write_addrs(&mut self, addr: Address, buf: &[Address]) -> Result<(), MemoryError> {
        let ptr = self
            .memory
            .address_to_pointer_mut::<*const ()>(self.slot, addr, buf.len())?;
        let data = unsafe { slice::from_raw_parts_mut(ptr, buf.len()) };
        for (dst, src) in data.iter_mut().zip(buf.iter().copied()) {
            *dst = self.memory.unchecked_address_to_pointer(src);
        }
        Ok(())
    }
}
