//! Implementation of `Memory` for a loaded DLL.
//!
//! The DLL is loaded into memory and is treated as a slot (the base slot).
//! Backup slots are allocated as buffers that can be copied to and from the
//! .data and .bss sections of the DLL. DLL functions can only be run on the
//! base slot.

pub use error::*;
pub use memory::*;

mod error;
mod layout;
mod memory;
