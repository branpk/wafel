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

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use error::*;
pub use game::*;
pub use timeline::*;
pub use wafel_data_type::{Address, FloatValue, IntValue, Value};

mod data_cache;
mod error;
mod game;
mod timeline;
