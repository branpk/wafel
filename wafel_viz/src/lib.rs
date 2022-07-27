#![feature(generic_associated_types)]
#![warn(missing_docs, missing_debug_implementations)]
#![allow(missing_docs)] // FIXME

pub use camera_control::*;
pub use config::*;
pub use error::*;
pub use sm64_gfx_render::*;

mod camera_control;
mod config;
mod custom_renderer;
mod error;
mod f3d_builder;
mod sm64_gfx_render;
