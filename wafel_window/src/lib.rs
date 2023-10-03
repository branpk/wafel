//! A crate for creating a window and rendering with wgpu, with support for egui
//! and [wafel_viz].
//!
//! This crate also initializes logging to a file and stderr.
//!
//! # Example
//! ```no_run
//! // This prevents the console window from appearing on Windows in release mode.
//! #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//!
//! let config = wafel_window::Config::new().with_title("Minimal example");
//!
//! wafel_window::run(&config, move |env| {
//!     let ctx = env.egui_ctx();
//!
//!     egui::CentralPanel::default().show(ctx, |ui| {
//!         ui.label(format!("{:#?}", env.config()));
//!         ui.label(format!("{:.3} mspf = {:.1} fps", env.mspf(), env.fps()));
//!     });
//! });
//! ```
//!
//! # Features
//! This crate has two optional features, both disabled by default:
//! - `wafel_viz`: Enables drawing [wafel_viz] visualizations to the window.
//! - `image`: Enables loading icons from .ico files (Windows only).

#![warn(rust_2018_idioms, missing_debug_implementations, missing_docs)]
#![allow(clippy::too_many_arguments)]

pub use config::*;
pub use window_env::*;

mod config;
mod container;
mod egui_state;
mod fps_counter;
mod logging;
mod wgpu_util;
mod window;
mod window_env;

/// Initializes logging, opens a window and runs the application.
///
/// This function does not return.
pub fn run(config: &Config, draw: impl FnMut(&dyn WindowEnv) + 'static) {
    logging::init(&config.log_file_path());

    logging::print_to_log_file(&"-".repeat(80));
    if !config.title().is_empty() {
        tracing::info!("{}", config.title());
    }
    tracing::info!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    if config.hot_reload_subscriber().is_some() {
        tracing::info!("Hot reload enabled");
    }

    window::open_window_and_run(config, draw);
}
