//! Logic and UI for the Wafel application.

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

use app::App;
use log::LevelFilter;
use wafel_graphics::run_wafel_app;

mod app;
mod config;
mod frame_slider;
mod input_text_with_error;
mod joystick_control;
mod object_slots;
mod project;
mod variable_value;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module("wgpu_core::device", LevelFilter::Warn)
        .init(); // TODO: Replace with log file

    let mut app = App::open();
    run_wafel_app(Box::new(move |ui| app.render(&ui)));
}
