//! The executable for the main Wafel GUI.

#![warn(missing_docs, missing_debug_implementations)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::env::WafelEnv;

mod app;
mod egui_state;
mod env;
mod logging;
mod window;

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

    let title = format!("Wafel {}", env.wafel_version());
    window::run_app::<app::WafelApp>(env, &title);
}
