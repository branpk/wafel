use crate::error::Error;
use pyo3::{create_exception, exceptions::PyException, prelude::*};
use std::backtrace::BacktraceStatus;

create_exception!(wafel, WafelError, PyException);

impl From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        // TODO: Include backtrace in log
        if let BacktraceStatus::Captured = err.backtrace.status() {
            eprintln!("{}", err.backtrace);
        }
        PyErr::new::<WafelError, _>(err.to_string())
    }
}
