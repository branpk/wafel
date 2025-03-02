//! Environment for the Wafel application.

use std::{
    env,
    path::{Path, PathBuf},
    sync::Mutex,
};

use once_cell::sync::Lazy;
use sysinfo::{Pid, PidExt, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};
use wafel_app_ui::{Env, ProcessInfo};

use crate::{logging, WAFEL_VERSION};

#[derive(Debug)]
pub struct WafelEnv {
    log_file_path: PathBuf,
}

impl WafelEnv {
    pub fn create() -> Self {
        // root_dir is directory that configuration and log files should be saved.
        // In release mode, this is the directory containing the executable.
        let root_dir = if cfg!(debug_assertions) {
            env::current_dir().expect("failed to locate current working directory")
        } else {
            let mut path = env::current_exe().expect("failed to locate executable");
            path.pop();
            path
        };

        let log_file_path = root_dir.join("log.txt");

        Self { log_file_path }
    }

    /// Return the path to the log file.
    pub fn log_file_path(&self) -> &Path {
        self.log_file_path.as_path()
    }

    /// Return the current version of Wafel.
    pub fn wafel_version(&self) -> &str {
        WAFEL_VERSION
    }
}

static SYSTEM: Lazy<Mutex<System>> = Lazy::new(|| {
    Mutex::new(System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new()),
    ))
});

impl Env for WafelEnv {
    fn wafel_version(&self) -> &str {
        WAFEL_VERSION
    }

    fn log_file_path(&self) -> &Path {
        self.log_file_path.as_path()
    }

    fn processes(&self) -> Vec<ProcessInfo> {
        let mut system = SYSTEM.lock().unwrap();
        system.refresh_processes();

        system
            .processes()
            .into_iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string(),
            })
            .collect()
    }

    fn is_process_open(&self, pid: u32) -> bool {
        let mut system = SYSTEM.lock().unwrap();
        system.refresh_process(Pid::from_u32(pid))
    }

    fn take_recent_panic_details(&self) -> Option<String> {
        logging::take_recent_panic_details()
    }
}
