//! The executable for the main Wafel GUI.
//!
//! It is possible to hot reload the [wafel_app_ui] crate while this binary is
//! running if the `reload` feature is enabled.
//! Commands to run (in separate terminals):
//!
//! ```sh'
//! cargo run -p wafel_app --features reload
//! cargo watch -w wafel_app_ui -x 'build -p wafel_app_ui'
//! ```

#![warn(missing_docs, missing_debug_implementations)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::env::WafelEnv;

mod app;
mod egui_state;
mod env;
mod hot_reload;
mod logging;
mod window;

// Important: be sure to re-run `python build.py lock` after changing!
const WAFEL_VERSION: &'static str = "0.8.5";

fn main() {
    let env = WafelEnv::create();

    logging::init(env.log_file_path());

    logging::print_to_log_file(&"-".repeat(80));
    tracing::info!("Wafel {}", env.wafel_version());
    tracing::info!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    if cfg!(feature = "reload") {
        tracing::info!("Hot reload enabled");
    }

    let title = format!("Wafel {}", env.wafel_version());
    window::run_app::<app::WafelApp>(env, &title);
}
