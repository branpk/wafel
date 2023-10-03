#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time;

use wafel_api::{Game, Input};
use wafel_viz::{ObjectCull, VizConfig};
use wafel_window::Config;

fn main() {
    let mut game = unsafe { Game::new("libsm64/sm64_us") };

    for frame in 0..1400 {
        if frame % 2 == 1 {
            game.write("gControllerPads[0].button", game.constant("START_BUTTON"));
        }
        game.advance();
    }
    let mut last_update_time = time::Instant::now();

    let mut a_down = false;
    let mut b_down = false;
    let mut stick_x: i8 = 0;
    let mut stick_y: i8 = 0;

    let config = Config::new();
    wafel_window::run(&config, move |env| {
        let ctx = env.egui_ctx();

        if last_update_time.elapsed().as_secs_f32() >= 1.0 / 30.0 {
            last_update_time = time::Instant::now();

            let mut buttons = 0;
            if a_down {
                buttons |= game.constant("A_BUTTON").as_int() as u16;
            }
            if b_down {
                buttons |= game.constant("B_BUTTON").as_int() as u16;
            }
            game.set_input(Input {
                buttons,
                stick_x,
                stick_y,
            });

            game.advance();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("{:.3} mspf = {:.1} fps", env.mspf(), env.fps()));
            ui.label(format!("frame = {}", game.frame()));

            ui.checkbox(&mut a_down, "A");
            ui.checkbox(&mut b_down, "B");
            ui.add(egui::Slider::new(&mut stick_x, -127..=127).text("X"));
            ui.add(egui::Slider::new(&mut stick_y, -127..=127).text("Y"));

            let rect = ui.available_rect_before_wrap();

            if rect.width() as u32 > 0 && rect.height() as u32 > 0 {
                let render_data = game.render(&VizConfig {
                    screen_top_left: [rect.left() as u32, rect.top() as u32],
                    screen_size: [rect.width() as u32, rect.height() as u32],
                    object_cull: ObjectCull::ShowAll,
                    ..Default::default()
                });

                env.draw_viz(render_data);
            }
        });
    });
}
