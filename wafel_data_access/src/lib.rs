//! Reading and writing structured data to memory.
//!
//! This crate provides two ways to access data:
//! - Data paths, which dynamically look up individual variables from memory
//! - [DataReadable] and [DataWritable], which enable reading/writing using native Rust types
//!
//! # Data paths
//!
//! A data path describes a location in memory using a C-like syntax. It allows
//! pointer dereferencing, array indexing, and struct/union field accesses.
//!
//! There are two types of data paths:
//! - Global: a data path starting from a global variable address
//! - Local: a data path starting from a type, such as a specific struct
//!
//! It is possible to concatenate paths, either global + local or local + local, if the end type
//! of the first path and the start type of the second path match.
//!
//! The syntax mostly follows C syntax, e.g. `globalVariable.arrayField[3].x` for a global
//! path.
//! Local paths are similar, but the global variable is replaced with a type namespace and name,
//! e.g. `struct Foo.x`, `typedef Foo.x`.
//!
//! The crate documentation for `wafel_api` has more details about the syntax.
//!
//! # [DataReadable] example
//!
//! ```no_run
//! use wafel_data_access::{DataReadable, Reader, MemoryLayout, DataError};
//! use wafel_data_type::Address;
//! use wafel_memory::MemoryRead;
//!
//! #[derive(Debug, Clone, DataReadable)]
//! #[struct_name("Surface")]
//! struct Surface {
//!     normal: [f32; 3],
//!     vertex1: [i16; 3],
//!     vertex2: [i16; 3],
//!     vertex3: [i16; 3],
//! }
//!
//! fn read_surface(
//!     layout: &impl MemoryLayout,
//!     memory: &impl MemoryRead,
//!     addr: Address
//! ) -> Result<Surface, DataError> {
//!     let reader: Reader<Surface> = Surface::reader(layout)?;
//!     let surface: Surface = reader.read(memory, addr)?;
//!     Ok(surface)
//! }
//! ```

#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

pub use data_path::*;
pub use error::*;
pub use layout::*;
pub use traits::*;
pub use wafel_data_access_derive::*;

mod compile;
mod data_path;
mod data_path_cache;
mod error;
mod layout;
mod parse;
pub mod readers;
mod traits;
pub mod writers;
