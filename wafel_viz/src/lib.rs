#![feature(doc_cfg)]
#![warn(missing_docs, missing_debug_implementations)]
#![allow(missing_docs)] // FIXME

pub use config::*;
pub use error::*;
pub use render_data::*;
#[cfg(feature = "wgpu")]
#[doc(cfg(feature = "wgpu"))]
pub use renderer::VizRenderer;
pub use rotate_camera_control::*;

mod config;
mod error;
mod f3d_builder;
// mod ortho_camera_control;
mod ortho_camera_control;
mod render_data;
#[cfg(feature = "wgpu")]
mod renderer;
mod rotate_camera_control;
mod skybox;
mod sm64_gfx_render;
