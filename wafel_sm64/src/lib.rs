//! Utilities for reading SM64 data from memory.

#![warn(missing_docs, missing_debug_implementations)]

pub use error::*;
pub use frame_log::*;
pub use mario::*;
pub use object::*;
pub use surface::*;

mod error;
mod frame_log;
mod mario;
mod object;
mod surface;
