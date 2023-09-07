//! The executable for the main Wafel GUI.

#![warn(missing_docs, missing_debug_implementations)]

mod app;
mod env;
mod logging;
mod window;

fn main() {
    logging::init();

    let env = env::global_env().lock().unwrap();

    logging::print_to_log_file(&"-".repeat(80));
    tracing::info!("Wafel {}", env.wafel_version());
    tracing::info!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    window::run_app::<app::WafelApp>(&format!("Wafel {}", env.wafel_version()));
}
