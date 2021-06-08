//! Defines a mapping from global variables to types/values.
//!
//! A [DataLayout] is a mapping containing:
//! - Type definitions
//! - Global variable types
//! - Constant values
//!
//! This layout is used for accessing global data in Wafel. It can be constructed manually
//! or read automatically from the DWARF debugging info of a DLL.
//!
//! Some SM64 specific data cannot be parsed from DWARF but is useful to have in the layout.
//! To include these, use [load_sm64_extras].
//!
//! A json representation of this layout can be produced using the libsm64_layout executable.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use data_layout::*;
pub use dll_layout::*;
pub use error::*;
pub use sm64_extra::*;

mod data_layout;
mod dll_layout;
mod error;
mod sm64_extra;
