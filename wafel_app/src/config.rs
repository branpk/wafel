//! Configuration variables for the Wafel application.

use std::{env, path::PathBuf};

/// Return the directory that configuration and log files should be saved.
///
/// In release mode, this is the directory containing the executable.
pub fn root_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        env::current_dir().expect("failed to locate current working directory")
    } else {
        let mut path = env::current_exe().expect("failed to locate executable");
        path.pop();
        path
    }
}

/// Return the path to the log file.
pub fn log_file_path() -> PathBuf {
    root_dir().join("log.txt")
}

/// Return the current version of Wafel.
pub fn wafel_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
