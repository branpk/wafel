use once_cell::sync::{Lazy, OnceCell};
use std::{
    backtrace::Backtrace,
    fmt, fs,
    io::Write,
    panic::{self, PanicInfo},
    path::Path,
    sync::Mutex,
};
use tracing::{
    field::{Field, Visit},
    Event, Level, Subscriber,
};
use tracing_log::LogTracer;
use tracing_subscriber::{
    layer::Context, prelude::__tracing_subscriber_SubscriberExt, registry::LookupSpan, Layer,
};

static LOG_FILE: OnceCell<Mutex<fs::File>> = OnceCell::new();
static RECENT_PANIC_DETAILS: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn init(log_file_path: &Path) {
    let log_file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(log_file_path)
        .expect("failed to open log file");
    LOG_FILE
        .set(Mutex::new(log_file))
        .expect("called logging::init more than once");

    panic::set_hook(Box::new(panic_hook));
    LogTracer::init().unwrap();
    tracing::subscriber::set_global_default(tracing_subscriber::Registry::default().with(LogLayer))
        .unwrap();
}

pub fn print_to_log_file(line: &str) {
    let mut log_file = LOG_FILE
        .get()
        .expect("missing call to logging::init")
        .lock()
        .unwrap();

    writeln!(log_file, "{}", line).expect("failed to write to log file");
    log_file.flush().expect("failed to flush log file");
}

fn log_callback(level: Level, message: &str) {
    if level <= Level::INFO {
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();

        let line = format!("[{}] [{}] {}", timestamp, level, message);

        eprintln!("{}", line);
        print_to_log_file(&line);
    }
}

pub fn take_recent_panic_details() -> Option<String> {
    RECENT_PANIC_DETAILS.lock().unwrap().take()
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

    *RECENT_PANIC_DETAILS.lock().unwrap() = Some(panic_details.clone());
    tracing::error!("{}", panic_details);
}

struct LogLayer;

#[derive(Default)]
struct MessageVisitor {
    message: String,
    log_target: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "log.target" {
            self.log_target = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
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

        let span = if let Some(scope) = ctx.event_scope(event) {
            format!(
                "[{}] ",
                scope
                    .from_root()
                    .map(|span| span.name())
                    .collect::<Vec<_>>()
                    .join(".")
            )
        } else {
            String::new()
        };

        let metadata = event.metadata();

        let target = visitor
            .log_target
            .unwrap_or_else(|| metadata.target().to_string());

        if target.starts_with("wgpu") && *metadata.level() >= Level::INFO {
            return;
        }

        let message = format!("{}[{}] {}", span, target, visitor.message);

        log_callback(*metadata.level(), &message);
    }
}
