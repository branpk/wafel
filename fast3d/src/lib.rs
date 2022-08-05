//! Decoding, interpreting, and rendering of Nintendo 64 Fast3D commands.
//!
//! Note: this module is not currently intended to be a complete and accurate implementation.
//! Several commands are unimplemented in both [decode] and [interpret].
//!
//! This crate requires nightly to compile.

#![feature(stmt_expr_attributes)]
#![feature(doc_cfg)]
#![feature(generic_associated_types)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![allow(clippy::map_entry, clippy::needless_range_loop)]

pub use error::*;

pub mod cmd;
pub mod decode;
mod error;
mod f3d_render_data;
pub mod interpret;
#[cfg(any(feature = "wgpu", doc))]
#[doc(cfg(feature = "wgpu"))]
pub mod render;
mod trig_tables;
pub mod util;
