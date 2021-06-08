#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use controller::*;
pub use timeline::*;

mod controller;
mod slots;
mod timeline;
