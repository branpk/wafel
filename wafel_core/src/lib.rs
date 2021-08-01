//! Rust code for Wafel.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
#![feature(try_blocks)]
#![feature(backtrace)]

pub use graphics::*;
pub use sm64::*;

mod error;
mod geo;
mod graphics;
mod python;
mod sm64;
