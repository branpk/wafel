//! Representation of Wafel data types and values, mostly corresponding to C data types.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use data_type::*;
pub use error::*;
pub use value::*;

mod data_type;
mod error;
pub mod shallow;
mod value;
