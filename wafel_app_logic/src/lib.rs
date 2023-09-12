//! The logic and UI for the main Wafel application.
//!
//! For testing purposes, this crate doesn't directly access the file system and
//! is agnostic to the window/graphics backend.
//! These operations are done indirectly through the [Env] trait, which can be
//! overriden as needed.

#![warn(missing_docs, missing_debug_implementations)]

pub use env::*;
pub use wafel::*;

mod env;
mod root;
mod wafel;
mod workspace;
mod workspace_mode;
