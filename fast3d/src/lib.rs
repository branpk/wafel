//! Decoding, interpreting, and rendering of Nintendo 64 Fast3D commands.
//!
//! There are three modules:
//! - [decode] parses raw commands into a structured [decode::F3DCommand]
//! - [interpret] processes a display list into a [interpret::F3DRenderData] which is
//!   straightforward to write a renderer for
//! - [render] is a renderer implementation using wgpu. It is only available if the
//!   `wgpu` feature is enabled
//!
//! Note: this module is not currently intended to be a complete and accurate implementation.
//! Several commands are unimplemented in both [decode] and [interpret].
//!
//! This crate requires nightly to compile.

#![feature(stmt_expr_attributes)]
#![feature(doc_cfg)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![allow(clippy::map_entry, clippy::needless_range_loop)]

pub mod decode;
mod f3d_render_data;
pub mod interpret;
#[cfg(any(feature = "wgpu", doc))]
#[doc(cfg(feature = "wgpu"))]
pub mod render;
pub mod util;
