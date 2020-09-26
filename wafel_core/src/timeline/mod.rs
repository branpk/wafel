//! The core abstraction for random access to frames in a simulation (rewinding etc).

pub use state::*;
pub use timeline_impl::*;

mod data_cache;
mod slot_manager;
mod slot_state_impl;
mod state;
mod timeline_impl;
