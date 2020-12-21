use crate::error::Error;
use pyo3::{create_exception, exceptions::PyException, prelude::*};
use std::{
    backtrace::{Backtrace, BacktraceStatus},
    panic::{self, PanicInfo},
};

use super::log;

create_exception!(wafel, WafelError, PyException);

impl From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        let message = if let BacktraceStatus::Captured = err.backtrace.status() {
            format!("{}\n{}", err, err.backtrace)
        } else {
            err.to_string()
        };
        PyErr::new::<WafelError, _>(message)
    }
}

pub fn init() {
    panic::set_hook(Box::new(panic_hook));
}

fn panic_hook(info: &PanicInfo<'_>) {
    let location = info.location().unwrap();
    let msg = match info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match info.payload().downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<Any>",
        },
    };
    let backtrace = Backtrace::force_capture();

    let panic_details = format!("Panicked at {}: {}\n{}", location, msg, backtrace);

    let result = log::error_acquire(format!("Panic details:\n{}", panic_details));
    if result.is_err() {
        eprintln!("Failed to log panic details:\n{}", panic_details);
    }
}
