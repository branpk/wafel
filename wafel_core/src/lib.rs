//! Rust code for Wafel.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
#![feature(try_blocks)]
#![feature(backtrace)]

pub mod data_path;
pub mod dll;
pub mod error;
pub mod geo;
pub mod graphics;
pub mod memory;
pub mod python;
pub mod sm64;
pub mod timeline;
