//! Reading and writing structured data to memory.
//!
//! This crate provides two ways to access data:
//! - Data paths, which dynamically look up individual variables from memory
//! - [DataReadable], which enables reading into Rust structs
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
//! #[derive(Debug, Clone, DataReadable)]
//! struct Surface {
//!     normal: [f32; 3],
//!     vertex1: [i16; 3],
//!     vertex2: [i16; 3],
//!     vertex3: [i16; 3],
//! }
//!
//! let reader: Reader<Surface> = Surface::reader(&layout)?;
//! let surface: Surface = reader.read(&memory, addr)?;
//! ```

#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

pub use data_path_types::*;
pub use error::*;
pub use layout::*;
pub use read_write::{DataReadable, DataReader, Reader};
pub use wafel_data_access_derive::*;

mod compile;
mod data_path_cache;
mod data_path_types;
mod error;
mod layout;
mod parse;
mod read_write;
