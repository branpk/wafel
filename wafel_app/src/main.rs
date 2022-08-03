//! Logic and UI for the Wafel application.

#![warn(missing_docs, missing_debug_implementations)]

use app::App;
use log::LevelFilter;
use wafel_graphics::run_wafel_app;

mod app;
mod config;
mod frame_sheet;
mod frame_slider;
mod game_view;
mod joystick_control;
mod object_slots;
mod project;
mod tabs;
mod variable_explorer;
mod variable_value;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter_module("wgpu_core::device", LevelFilter::Warn)
        .init(); // TODO: Replace with log file

    let mut app = App::open();
    run_wafel_app(Box::new(move |ui| app.render(&ui)));
}
