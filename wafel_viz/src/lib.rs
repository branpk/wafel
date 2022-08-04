#![feature(generic_associated_types)]
#![warn(missing_docs, missing_debug_implementations)]
#![allow(missing_docs)] // FIXME

pub use config::*;
pub use error::*;
pub use render_data::*;
pub use renderer::*;
pub use rotate_camera_control::*;

mod config;
mod error;
mod f3d_builder;
// mod ortho_camera_control;
mod render_data;
mod renderer;
mod rotate_camera_control;
mod sm64_gfx_render;
