//! Abstraction for game memory.

pub use data_layout::*;
pub use error::*;
pub use memory_trait::*;
pub use value::*;

mod data_layout;
mod error;
mod memory_trait;
mod value;
