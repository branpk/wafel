#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::f32::consts::PI;

use ultraviolet::{Vec3, Vec4};
use wafel_viz::{Camera, PointElement, Rect2, TriangleElement, VizScene};
use wafel_window::AppConfig;

fn main() {
    let config = AppConfig::new().with_title("Camera bounds");

    let mut point: Vec3 = Vec3::zero();
    let mut cam_pos: Vec3 = Vec3::zero();
    let mut cam_angle: Vec3 = Vec3::zero();
    let mut perspective: bool = false;
    let mut look_at: bool = false;

    wafel_window::run(&config, move |env| {
        let ctx = env.egui_ctx();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().slider_width = ui.available_width() - 150.0;

                ui.add(egui::Slider::new(&mut point.x, -10.0..=10.0).text("point x"));
                ui.add(egui::Slider::new(&mut point.y, -10.0..=10.0).text("point y"));
                ui.add(egui::Slider::new(&mut point.z, -10.0..=10.0).text("point z"));

                ui.add_space(5.0);
                ui.add(egui::Slider::new(&mut cam_pos.x, -10.0..=10.0).text("cam x"));
                ui.add(egui::Slider::new(&mut cam_pos.y, -10.0..=10.0).text("cam y"));
                ui.add(egui::Slider::new(&mut cam_pos.z, -10.0..=10.0).text("cam z"));

                ui.add_space(5.0);
                ui.add(egui::Slider::new(&mut cam_angle.x, -PI..=PI).text("cam pitch"));
                ui.add(egui::Slider::new(&mut cam_angle.y, -PI..=PI).text("cam yaw"));
                ui.add(egui::Slider::new(&mut cam_angle.z, -PI..=PI).text("cam roll"));

                ui.add_space(5.0);
                ui.checkbox(&mut perspective, "perspective");
                ui.checkbox(&mut look_at, "look at");
            });

            let rect = ui.available_rect_before_wrap();

            let mut camera = if perspective {
                Camera::perspective(45.0, rect.width() / rect.height(), 0.1, 8.0)
            } else {
                Camera::orthographic(10.0, 10.0, 0.1, 8.0)
            };

            if look_at {
                camera = camera.look_at_with_roll(cam_pos, point, cam_angle.z);
            } else {
                camera = camera
                    .translate(cam_pos)
                    .rotate(cam_angle.x, cam_angle.y, cam_angle.z);
            }

            let mut scene = VizScene::new();
            scene.set_viewport_logical(Rect2::from_min_and_max(
                <[f32; 2]>::from(rect.min).into(),
                <[f32; 2]>::from(rect.max).into(),
            ));
            scene.set_camera(camera);

            let square_width = 1.0;
            let square_color = Vec4::new(0.8, 0.8, 0.8, 1.0);
            let point_size = 5.0;
            let point_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
            scene.add(
                TriangleElement::new([
                    point + Vec3::new(-square_width / 2.0, -square_width / 2.0, 0.0),
                    point + Vec3::new(square_width / 2.0, -square_width / 2.0, 0.0),
                    point + Vec3::new(-square_width / 2.0, square_width / 2.0, 0.0),
                ])
                .with_color(square_color),
            );
            scene.add(
                TriangleElement::new([
                    point + Vec3::new(square_width / 2.0, -square_width / 2.0, 0.0),
                    point + Vec3::new(-square_width / 2.0, square_width / 2.0, 0.0),
                    point + Vec3::new(square_width / 2.0, square_width / 2.0, 0.0),
                ])
                .with_color(square_color),
            );
            scene.add(
                PointElement::new(point)
                    .with_size(point_size)
                    .with_color(point_color),
            );

            env.draw_viz(scene);
        });
    });
}
