use std::{env, path::PathBuf};

pub fn root_dir() -> PathBuf {
    if cfg!(debug_assertions) {
        env::current_dir().expect("failed to locate current working directory")
    } else {
        let mut path = env::current_exe().expect("failed to locate executable");
        path.pop();
        path
    }
}

pub fn log_file_path() -> PathBuf {
    root_dir().join("log.txt")
}

pub fn wafel_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
