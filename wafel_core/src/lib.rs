//! Rust code for Wafel.

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_idioms)]
#![allow(clippy::float_cmp)]
#![feature(try_blocks)]
#![feature(backtrace)]
#![feature(option_expect_none)]
#![feature(range_is_empty)]
#![feature(inner_deref)]

pub mod data_path;
pub mod dll;
pub mod error;
pub mod geo;
pub mod graphics;
pub mod memory;
pub mod python;
pub mod sm64;
pub mod timeline;
