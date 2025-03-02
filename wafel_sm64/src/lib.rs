//! Utilities for reading SM64 data from memory.

#![warn(missing_docs, missing_debug_implementations)]

pub use error::*;
pub use frame_log::*;
pub use mario::*;
pub use object::*;
pub use segment_table::*;
pub use surface::*;

mod error;
mod frame_log;
pub mod gfx;
mod mario;
mod object;
mod segment_table;
mod surface;
