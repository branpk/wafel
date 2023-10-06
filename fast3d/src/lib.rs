//! Decoding, interpreting, and rendering of Nintendo 64 Fast3D commands.
//!
//! Note: this module is not currently intended to be a complete and accurate implementation.
//! Several commands are unimplemented in both [decode] and [interpret].

#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![allow(
    clippy::map_entry,
    clippy::needless_range_loop,
    clippy::manual_range_patterns
)]

pub use error::*;

pub mod cmd;
pub mod decode;
mod error;
mod f3d_render_data;
pub mod interpret;
#[cfg(feature = "wgpu")]
pub mod render;
mod trig_tables;
pub mod util;
