#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

pub use dll_memory::*;
pub use error::*;
pub use traits::*;

mod dll_memory;
mod dll_slot_impl;
mod error;
mod traits;
