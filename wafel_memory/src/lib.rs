//! Abstractions for reading and writing values to game memory.
//!
//! This crate defines the [MemoryRead] and [MemoryWrite] traits which can be
//! used to read/write [Value](wafel_data_type::Value)s to arbitrary addresses.
//!
//! It also defines the [GameMemory] trait which distinguishes between static
//! and non-static memory and provides state saving/loading functionality.
//!
//! Finally it provides [DllGameMemory] which implements [GameMemory] using a
//! game DLL, and [EmuMemory] which attaches to a running emulator.

#![warn(missing_docs, missing_debug_implementations)]

pub use dll_memory::*;
pub use emu_memory::*;
pub use error::*;
pub use remote_dll_memory::*;
pub use traits::*;

mod dll_memory;
mod dll_slot_impl;
mod emu_memory;
mod error;
mod remote_dll_memory;
mod traits;
mod unique_dll;
