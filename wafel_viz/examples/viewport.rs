#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wafel_api::{Game, ObjectCull, SurfaceMode, VizConfig};
use wafel_viz::Rect2;
use wafel_window::AppConfig;

fn main() {
    let app_config = AppConfig::new().with_title("Viewport");

    let mut game = unsafe { Game::new("libsm64/sm64_us") };
    for frame in 0..1630 {
        if frame % 2 == 1 {
            game.write("gControllerPads[0].button", game.constant("START_BUTTON"));
        } else {
            game.write("gControllerPads[0].button", game.constant("A_BUTTON"));
        }
        game.advance();
    }

    let mut top = 0.0;
    let mut bottom = 600.0;
    let mut left = 0.0;
    let mut right = 800.0;

    wafel_window::run(&app_config, move |env| {
        egui::CentralPanel::default().show(env.egui_ctx(), |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().slider_width = ui.available_width() - 150.0;
                ui.add(egui::Slider::new(&mut top, -100.0..=700.0).text("top"));
                ui.add(egui::Slider::new(&mut bottom, -100.0..=700.0).text("bottom"));
                ui.add(egui::Slider::new(&mut left, -100.0..=900.0).text("left"));
                ui.add(egui::Slider::new(&mut right, -100.0..=900.0).text("right"));
            });

            let viz_config = VizConfig {
                screen_top_left: [left as i32, top as i32],
                screen_size: [(right - left) as i32, (bottom - top) as i32],
                object_cull: ObjectCull::ShowAll,
                surface_mode: SurfaceMode::Physical,
                ..Default::default()
            };

            let mut scene = game.render(&viz_config);
            scene.set_viewport_logical(Rect2::from_min_and_max(
                [left, top].into(),
                [right, bottom].into(),
            ));

            env.draw_viz(scene);
        });
    });
}
