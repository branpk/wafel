use std::{
    collections::HashMap,
    fmt,
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

use once_cell::sync::OnceCell;

use crate::{MemoryError, MemoryInitError};

// EmuMemory doesn't implement GameMemory because it isn't able to make any guarantees about
// how/when the process writes to the base slot.
// In the future, Wafel could have an embedded emulator that it can control, which
// could implement GameMemory.

/// A backup slot for [EmuMemory].
///
/// This can be used for saving and loading save states.
pub struct EmuSlot {
    memory_id: Arc<()>,
    id: usize,
    buffer: Vec<u8>,
}

impl fmt::Debug for EmuSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmuSlot")
            .field("memory_id", &self.memory_id)
            .field("id", &self.id)
            .finish()
    }
}

/// Memory view for reading/writing to a running emulator.
#[derive(Debug)]
pub struct EmuMemory {
    id: Arc<()>,
    pid: i32,
    base_address: usize,
    memory_size: usize,
    next_buffer_id: AtomicUsize,
}

impl EmuMemory {
    /// Attach to a running emulator and return a [EmuMemory] representing a read/write view
    /// of the process's memory.
    pub fn attach(pid: i32, base_address: usize, memory_size: usize) -> Self {
        Self {
            id: Arc::new(()),
            pid,
            base_address,
            memory_size,
            next_buffer_id: AtomicUsize::new(1),
        }
    }

    fn validate_slot(&self, slot: &EmuSlot) {
        assert!(
            Arc::ptr_eq(&self.id, &slot.memory_id),
            "slot is not owned by this emu memory"
        );
    }
}
