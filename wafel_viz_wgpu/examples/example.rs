#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wafel_viz::{Line, VizScene};
use wafel_window::AppConfig;

fn main() {
    let config = AppConfig::new().with_title("Viz Example");

    wafel_window::run(&config, move |env| {
        let screen_rect = env.egui_ctx().screen_rect();

        let mut scene = VizScene::new();
        scene.set_viewport_logical(
            [screen_rect.left() as u32, screen_rect.top() as u32],
            [screen_rect.width() as u32, screen_rect.height() as u32],
        );
        scene.add(Line::new([[-1.0, -1.0, -0.1], [1.0, 1.0, 1.1]]));

        env.draw_viz(scene);
    });
}
