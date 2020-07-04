//! Rust code for Wafel.

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![feature(try_blocks)]
#![feature(ptr_wrapping_offset_from)]
#![feature(backtrace)]

// TODO: Fix exports

pub mod data_path;
pub mod dll;
pub mod error;
pub mod memory;
pub mod python;
pub mod sm64;
pub mod timeline;
