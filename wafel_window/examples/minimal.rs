// This prevents the console window from appearing on Windows in release mode.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wafel_window::Config;

fn main() {
    let config = Config::new().with_title("Minimal example");

    wafel_window::run(&config, move |env| {
        let ctx = env.egui_ctx();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("{:#?}", env.config()));
            ui.label(format!("{:.3} mspf = {:.1} fps", env.mspf(), env.fps()));
        });
    });
}
