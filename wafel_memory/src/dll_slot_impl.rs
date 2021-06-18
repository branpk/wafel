use std::{fmt, slice};

use wafel_layout::DllSegment;

#[derive(Debug)]
pub(crate) enum SlotImpl {
    /// Base slot, backed by DLL memory.
    Base(BaseSlot),
    /// Buffer slot, allocated by user.
    Buffer(BufferSlot),
}

impl SlotImpl {
    pub(crate) fn memory_id(&self) -> usize {
        match self {
            Self::Base(slot) => slot.memory_id,
            Self::Buffer(slot) => slot.memory_id,
        }
    }

    pub(crate) unsafe fn segment(&self, index: usize) -> Option<&[u8]> {
        match self {
            Self::Base(slot) => slot.segment(index),
            Self::Buffer(slot) => slot.segment(index),
        }
    }

    pub(crate) unsafe fn segment_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        match self {
            Self::Base(slot) => slot.segment_mut(index),
            Self::Buffer(slot) => slot.segment_mut(index),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BasePointer(pub(crate) *mut u8);

// The DLL's memory is always accessed via a Slot object (read-write) or
// a static address (read-only), so Rust's borrow rules enforce safe accesses.
unsafe impl Send for BasePointer {}
unsafe impl Sync for BasePointer {}

/// A backup buffer that can hold the data segments of the DLL.
#[derive(Debug)]
pub(crate) struct BufferSlot {
    memory_id: usize,
    id: usize,
    segments: Vec<SegmentWrapper>,
}

impl BufferSlot {
    pub(crate) fn new(memory_id: usize, id: usize, segments: Vec<Vec<u8>>) -> Self {
        Self {
            memory_id,
            id,
            segments: segments.into_iter().map(SegmentWrapper).collect(),
        }
    }

    pub(crate) fn segment(&self, index: usize) -> Option<&[u8]> {
        self.segments.get(index).map(|seg| seg.0.as_slice())
    }

    pub(crate) fn segment_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        self.segments.get_mut(index).map(|seg| seg.0.as_mut_slice())
    }
}

struct SegmentWrapper(Vec<u8>);

impl fmt::Debug for SegmentWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<segment of size {}>", self.0.len())
    }
}

/// The slot representing the DLL's loaded memory.
#[derive(Debug)]
pub(crate) struct BaseSlot {
    memory_id: usize,
    base_pointer: BasePointer,
    base_size: usize,
    data_segments: Vec<DllSegment>,
}

impl BaseSlot {
    pub(crate) fn new(
        memory_id: usize,
        base_pointer: BasePointer,
        base_size: usize,
        data_segments: Vec<DllSegment>,
    ) -> Self {
        Self {
            memory_id,
            base_pointer,
            base_size,
            data_segments,
        }
    }

    /// # Safety
    /// No other pointers should write to the DLL memory while the slice is live.
    pub(crate) unsafe fn segment(&self, index: usize) -> Option<&[u8]> {
        let info = self.data_segments.get(index)?;
        let segment_pointer = self
            .base_pointer
            .0
            .wrapping_add(info.virtual_address as usize);
        Some(slice::from_raw_parts(
            segment_pointer,
            info.virtual_size as usize,
        ))
    }

    /// # Safety
    /// No other pointers should access the DLL memory while the slice is live.
    pub(crate) unsafe fn segment_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        let info = self.data_segments.get(index)?;
        let segment_pointer = self
            .base_pointer
            .0
            .wrapping_add(info.virtual_address as usize);
        Some(slice::from_raw_parts_mut(
            segment_pointer,
            info.virtual_size as usize,
        ))
    }
}
