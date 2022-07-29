#![feature(generic_associated_types)]
#![warn(missing_docs, missing_debug_implementations)]
#![allow(missing_docs)] // FIXME

pub use camera_control::*;
pub use config::*;
pub use error::*;
pub use render_data::*;
pub use renderer::*;

mod camera_control;
mod config;
mod error;
mod f3d_builder;
mod render_data;
mod renderer;
mod sm64_gfx_render;
