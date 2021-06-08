//! Abstractions for reading and writing values to game memory.
//!
//! This crate defines the [MemoryRead] and [MemoryWrite] traits which can be
//! used to read/write [Value](wafel_data_type::Value)s to arbitrary addresses.
//!
//! It also defines the [GameMemory] trait which distinguishes between static
//! and non-static memory and provides state saving/loading functionality.
//!
//! Finally it provides [DllGameMemory] which implements [GameMemory] using a
//! game DLL.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

pub use dll_memory::*;
pub use error::*;
pub use traits::*;

mod dll_memory;
mod dll_slot_impl;
mod error;
mod traits;
