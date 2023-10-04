//! Renderer for [wafel_viz] using wgpu.
//!
//! This can be used directly or automatically via `wafel_window`.

#![warn(rust_2018_idioms, missing_debug_implementations, missing_docs)]

pub use renderer::VizRenderer;

mod data;
mod pipelines;
mod renderer;
