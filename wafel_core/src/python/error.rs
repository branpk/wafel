use crate::error::Error;
use pyo3::{create_exception, exceptions::PyException, prelude::*};
use std::{
    backtrace::{Backtrace, BacktraceStatus},
    fmt::Debug,
    panic::{self, PanicInfo},
};
use tracing::{
    field::{Field, Visit},
    Event, Level, Subscriber,
};
use tracing_log::LogTracer;
use tracing_subscriber::{
    layer::Context, prelude::__tracing_subscriber_SubscriberExt, registry::LookupSpan, Layer,
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
    LogTracer::init().unwrap();
    tracing::subscriber::set_global_default(tracing_subscriber::Registry::default().with(LogLayer))
        .unwrap();
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
    log::error_acquire(format!("Panic details:\n{}", panic_details));
}

pub struct LogLayer;

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }
}

impl<S> Layer<S> for LogLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let mut span_string = String::new();
        for span in ctx.scope() {
            if !span_string.is_empty() {
                span_string.push_str(" | ");
            }
            span_string.push_str(span.name());
        }

        let metadata = event.metadata();
        let module = metadata.module_path().unwrap_or("no module");

        let message = format!("{}, {}: {}", span_string, module, visitor.message);

        match *metadata.level() {
            Level::ERROR => log::error_acquire(message),
            Level::WARN => log::warn_acquire(message),
            Level::INFO => {}  //log::info_acquire(message),
            Level::DEBUG => {} //log::debug_acquire(message),
            Level::TRACE => {}
        }
    }
}
