//! Provides access to the log file.

#![allow(unused)]

use pyo3::prelude::*;

fn call_log_method(py: Python<'_>, method: &str, message: impl Into<String>) -> PyResult<()> {
    let log = PyModule::import(py, "wafel.log")?;
    log.call_method1(method, (message.into(),))?;
    Ok(())
}

/// Print a debug message.
pub fn debug(py: Python<'_>, message: impl Into<String>) {
    let _ = call_log_method(py, "debug", message);
}

/// Print a debug message.
pub fn debug_acquire(message: impl Into<String>) {
    Python::with_gil(|py| debug(py, message));
}

/// Print an info message.
pub fn info(py: Python<'_>, message: impl Into<String>) {
    let _ = call_log_method(py, "info", message);
}

/// Print an info message.
pub fn info_acquire(message: impl Into<String>) {
    Python::with_gil(|py| info(py, message));
}

/// Print a warning.
pub fn warn(py: Python<'_>, message: impl Into<String>) {
    let _ = call_log_method(py, "warn", message);
}

/// Print a warning.
pub fn warn_acquire(message: impl Into<String>) {
    Python::with_gil(|py| warn(py, message));
}

/// Print an error.
pub fn error(py: Python<'_>, message: impl Into<String>) -> PyResult<()> {
    call_log_method(py, "error", message)
}

/// Print an error.
pub fn error_acquire(message: impl Into<String>) -> PyResult<()> {
    Python::with_gil(|py| error(py, message))
}
