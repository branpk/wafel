//! Provides access to the log file.

#![allow(unused)]

use pyo3::prelude::*;

fn call_log_method(py: Python<'_>, method: &str, message: impl Into<String>) {
    let message_str = message.into();
    let result: PyResult<()> = try {
        let log = PyModule::import(py, "wafel.log")?;
        log.call_method1(method, (&message_str,))?;
    };
    if result.is_err() {
        eprintln!(
            "Failed to log message (level = {}):\n{}",
            method, message_str
        );
    }
}

/// Print a debug message.
pub fn debug(py: Python<'_>, message: impl Into<String>) {
    call_log_method(py, "debug", message);
}

/// Print a debug message.
pub fn debug_acquire(message: impl Into<String>) {
    Python::with_gil(|py| debug(py, message));
}

/// Print an info message.
pub fn info(py: Python<'_>, message: impl Into<String>) {
    call_log_method(py, "info", message);
}

/// Print an info message.
pub fn info_acquire(message: impl Into<String>) {
    Python::with_gil(|py| info(py, message));
}

/// Print a warning.
pub fn warn(py: Python<'_>, message: impl Into<String>) {
    call_log_method(py, "warn", message);
}

/// Print a warning.
pub fn warn_acquire(message: impl Into<String>) {
    Python::with_gil(|py| warn(py, message));
}

/// Print an error.
pub fn error(py: Python<'_>, message: impl Into<String>) {
    call_log_method(py, "error", message);
}

/// Print an error.
pub fn error_acquire(message: impl Into<String>) {
    Python::with_gil(|py| error(py, message));
}
