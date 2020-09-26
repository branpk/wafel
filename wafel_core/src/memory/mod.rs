//! Abstraction for game memory.

pub use data_layout::*;
pub use error::*;
pub use memory_trait::*;
pub use value::*;

mod data_layout;
pub mod data_type;
mod error;
mod memory_trait;
pub mod shallow_data_type;
mod value;
