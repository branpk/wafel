//! Logic and UI for the Wafel application.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

pub use app::App;

mod app;
mod config;
mod project;
