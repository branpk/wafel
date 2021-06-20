//! Logic and UI for the Wafel application.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

use app::App;
use wafel_graphics::run_wafel_app;

mod app;
mod config;
mod project;

fn main() {
    let mut app = App::open();
    run_wafel_app(Box::new(move |ui| app.render(&ui)));
}
