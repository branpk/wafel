#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use model::*;
pub use range_edit::{EditRange, EditRangeId};

mod model;
mod range_edit;
