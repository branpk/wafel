#![warn(rust_2018_idioms, missing_debug_implementations)]

// TODO: Documentation

pub use camera::*;
pub use element::*;
pub use scene::*;
pub use ultraviolet::{Mat4, Vec2, Vec3, Vec4};

mod camera;
mod element;
mod scene;
