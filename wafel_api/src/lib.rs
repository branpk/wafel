//! The main SM64 API for Wafel.
//!
//! This crate provides two different APIs: [Game] and [Timeline].
//!
//! The [Game] API uses a traditional frame advance / save state model.
//!
//! The [Timeline] API is higher level and manages save states internally. It allows accessing
//! arbitrary frames in any order. This is the API used by the Wafel application for its
//! rewind functionality.
//!
//! For now, I would recommend using the [Game] API for brute forcing purposes. The [Timeline]
//! API works, but the algorithm is currently optimized for Wafel's UI, so it may not be as
//! fast in a brute forcing setting.
//!
//! # Data paths
//!
//! Many API methods take in a string `path` parameter. This string uses a C-like syntax to
//! denote a location in memory.
//!
//! The following are all valid syntax:
//! - `gMarioState.pos`
//! - `gObjectPool[12].oPosX`
//! - `gMarioState[0].wall?.normal`
//! - `gMarioState.action & ACT_ID_MASK`
//!
//! Note:
//! - `p.x` automatically dereferences a pointer `p`. The syntax `p->x` can also be used.
//! - `*` is not used for pointer dereferencing. Instead you can use `[0]`, `->`, or `.`.
//! - A mask can be applied using `&`. The mask must be an integer literal or constant name.
//! - Array indices must be an integer literal or constant name.
//! - `?` denotes that a pointer may be null. If so, the entire expression returns `Value::None`.
//!   If `?` is not used, an error is thrown instead.
//!
//! Variable and constant names are automatically pulled from the decomp source code.
//! However, the names may be different if Wafel is out of date from decomp.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use data_type::*;
pub use error::*;
pub use game::*;
pub use object::*;
pub use surface::*;
pub use timeline::*;
pub use wafel_data_type::{
    Address, FloatType, FloatValue, IntType, IntValue, Value, ValueTypeError,
};

mod data_cache;
mod data_path_cache;
mod data_type;
mod error;
mod frame_log;
mod game;
mod mario;
mod object;
mod surface;
mod timeline;
