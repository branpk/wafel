//! The logic and UI for the main Wafel application.
//!
//! For testing purposes, this crate doesn't directly access the file system and
//! is agnostic to the window/graphics backend.
//! These operations are done indirectly through the [Env] trait, which can be
//! overriden as needed.
//!
//! It is possible to hot reload this crate while `wafel_app` is running by
//! rebuilding it while the app is running with the `reload` feature enabled.
//! Commands to run (in separate terminals):
//!
//! ```sh'
//! cargo run -p wafel_app --features reload
//! cargo watch -w wafel_app_ui -x "build -p wafel_app_ui"
//! ```

#![warn(missing_docs, missing_debug_implementations)]

pub use env::*;
pub use wafel::*;
use wafel_api::VizRenderData;

mod data_explorer;
mod emu_selector;
mod env;
mod error_boundary;
mod pane;
mod root;
mod wafel;
mod workspace;
mod workspace_mode;
mod workspace_root;

/// Render the Wafel UI and respond to user input events.
///
/// This function is no_mangle so that it can be hot reloaded.
#[no_mangle]
pub fn wafel_show(wafel: &mut Wafel, env: &dyn Env, ctx: &egui::Context) -> Vec<VizRenderData> {
    wafel.show(env, ctx)
}
