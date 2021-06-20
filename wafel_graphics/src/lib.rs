#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use imgui_render::*;
pub use window::*;

mod imgui_render;
mod window;
