use std::env;

use crate::logging::print_to_log_file;

pub mod config;
mod logging;

fn main() {
    logging::init();

    print_to_log_file(&"-".repeat(80));
    tracing::info!("Wafel X.X.X"); // TODO: Version number
    tracing::info!("Platform: {} {}", env::consts::OS, env::consts::ARCH)
}
