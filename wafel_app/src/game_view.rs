use std::{collections::HashSet, f32::consts::PI, ops::Deref, time::Instant};

use imgui::{self as ig, im_str};
use wafel_api::Timeline;
use wafel_core::{
    scene::{BirdsEyeCamera, Camera, RotateCamera, Scene, Viewport},
    Pipeline, SurfaceSlot, Variable,
};

// TODO: Rename to game_view_overlay. Reduce parameters to minimum (don't require full Model)
// TODO: Update to use nalgebra

#[derive(Debug, Clone, Default)]
struct MouseTracker {
    dragging: bool,
    mouse_down: bool,
    mouse_pos: [f32; 2],
}

impl MouseTracker {
    fn new() -> Self {
        Self::default()
    }

    fn is_mouse_in_window(&self, ui: &ig::Ui<'_>) -> bool {
        // TODO: global mouse capture
        // if not ig.global_mouse_capture():
        //   return False
        self.is_mouse_in_window_raw(ui)
    }

    fn is_mouse_in_window_raw(&self, ui: &ig::Ui<'_>) -> bool {
        let [wx, wy] = ui.window_pos();
        let [ww, wh] = ui.window_size();
        let [mx, my] = self.mouse_pos;
        mx >= wx && mx < wx + ww && my >= wy && my < wy + wh
    }

    fn update_drag_amount(&mut self, ui: &ig::Ui<'_>) -> [f32; 2] {
        let mouse_was_down = self.mouse_down;
        let last_mouse_pos = self.mouse_pos;
        self.mouse_down = ui.is_mouse_down(ig::MouseButton::Left); // TODO: && ig.global_mouse_capture();
        self.mouse_pos = ui.io().mouse_pos;

        if self.dragging {
            if !self.mouse_down {
                self.dragging = false;
            }
            return [
                self.mouse_pos[0] - last_mouse_pos[0],
                self.mouse_pos[1] - last_mouse_pos[1],
            ];
        }

        if !mouse_was_down
            && self.mouse_down
            && !ui.is_any_item_hovered()
            && self.is_mouse_in_window_raw(ui)
        {
            self.dragging = true;
        }

        [0.0, 0.0]
    }

    fn update_wheel_amount(&mut self, ui: &ig::Ui<'_>) -> f32 {
        if self.is_mouse_in_window(ui) {
            ui.io().mouse_wheel
        } else {
            0.0
        }
    }
}

fn angle_to_direction(pitch: f32, yaw: f32) -> [f32; 3] {
    [
        pitch.cos() * yaw.sin(),
        pitch.sin(),
        pitch.cos() * yaw.cos(),
    ]
}

fn direction_to_angle(dir: [f32; 3]) -> (f32, f32) {
    let xz = (dir[0].powi(2) + dir[2].powi(2)).sqrt();
    let pitch = f32::atan2(dir[1], xz);
    let yaw = f32::atan2(dir[0], dir[2]);
    (pitch, yaw)
}

fn viewport(ui: &ig::Ui<'_>, framebuffer_size: [f32; 2]) -> Viewport {
    Viewport {
        x: ui.window_pos()[0],
        y: ui.window_pos()[1],
        width: ui.window_size()[0],
        height: ui.window_size()[1],
    }
}

fn mario_pos(pipeline: &Pipeline, frame: u32) -> [f32; 3] {
    pipeline
        .timeline()
        .read(frame, "gMarioState.pos")
        .as_f32_3()
}

fn move_toward(current: [f32; 3], target: [f32; 3], delta: f32) -> [f32; 3] {
    let remaining = [
        target[0] - current[0],
        target[1] - current[1],
        target[2] - current[2],
    ];
    let distance = (remaining[0].powi(2) + remaining[1].powi(2) + remaining[2].powi(2)).sqrt();
    if distance <= delta + 0.001 {
        target
    } else {
        [
            current[0] + delta * remaining[0] / distance,
            current[1] + delta * remaining[1] / distance,
            current[2] + delta * remaining[2] / distance,
        ]
    }
}

fn normalized_mouse_pos(ui: &ig::Ui<'_>) -> Option<[f32; 2]> {
    // TODO: Global mouse capture
    // if not ig.global_mouse_capture():
    //     return None

    let window_pos = ui.window_pos();
    let window_size = ui.window_size();
    let mouse_pos = ui.io().mouse_pos;
    let mouse_pos = [
        mouse_pos[0] - window_pos[0],
        window_size[1] - mouse_pos[1] + window_pos[1],
    ];
    let mouse_pos = [
        2.0 * mouse_pos[0] / window_size[0] - 1.0,
        2.0 * mouse_pos[1] / window_size[1] - 1.0,
    ];
    if mouse_pos[0].abs() > 1.0 || mouse_pos[1].abs() > 1.0 {
        None
    } else {
        Some(mouse_pos)
    }
}

fn mouse_ray(ui: &ig::Ui<'_>, camera: &RotateCamera) -> Option<([f32; 3], [f32; 3])> {
    let window_size = ui.window_size();
    normalized_mouse_pos(ui).map(|mouse_pos| {
        let forward_dir = angle_to_direction(camera.pitch(), camera.yaw());
        let up_dir = angle_to_direction(camera.pitch() + PI / 2.0, camera.yaw());
        let right_dir = angle_to_direction(0.0, camera.yaw() - PI / 2.0);

        let top = (camera.fov_y / 2.0).tan();
        let right = top * window_size[0] / window_size[1];

        let mut mouse_dir: [f32; 3] = Default::default();
        for i in 0..3 {
            mouse_dir[i] = forward_dir[i]
                + top * mouse_pos[1] * up_dir[i]
                + right * mouse_pos[0] * right_dir[i];
        }
        let mag = mouse_dir.iter().map(|c| c.powi(2)).sum::<f32>().sqrt();
        let mouse_dir = [mouse_dir[0] / mag, mouse_dir[1] / mag, mouse_dir[2] / mag];

        ([camera.pos.x, camera.pos.y, camera.pos.z], mouse_dir)
    })
}

fn mouse_world_pos_birds_eye(ui: &ig::Ui<'_>, camera: &BirdsEyeCamera) -> Option<[f32; 2]> {
    let window_size = ui.window_size();
    normalized_mouse_pos(ui).map(|mouse_pos| {
        let world_span_x = camera.span_y;
        let world_span_z = camera.span_y * window_size[0] / window_size[1];
        [
            camera.pos[0] + mouse_pos[1] * world_span_x / 2.0,
            camera.pos[2] + mouse_pos[0] * world_span_z / 2.0,
        ]
    })
}

fn disableable_button(ui: &ig::Ui<'_>, label: &ig::ImStr, size: [f32; 2], enabled: bool) -> bool {
    if !enabled {
        let tok1 = ui.push_style_var(ig::StyleVar::Alpha(0.5));
        let tok2 = ui.push_style_colors(&[
            (
                ig::StyleColor::ButtonHovered,
                ui.style_color(ig::StyleColor::Button),
            ),
            (
                ig::StyleColor::ButtonActive,
                ui.style_color(ig::StyleColor::Button),
            ),
        ]);
        let result = ui.button(label, size);
        tok2.pop(ui);
        tok1.pop(ui);
        result
    } else {
        ui.button(label, size)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct GameViewRotate {
    mouse_state: MouseTracker,
    target: Option<[f32; 3]>,
    target_vel: Option<[f32; 3]>,
    pitch: f32,
    yaw: f32,
    zoom: f32,
    prev_frame_time: Instant,
    lock_to_in_game: bool,
}

impl Default for GameViewRotate {
    fn default() -> Self {
        Self::new()
    }
}

impl GameViewRotate {
    pub(crate) fn new() -> Self {
        Self {
            mouse_state: MouseTracker::new(),
            target: None,
            target_vel: None,
            pitch: 0.0,
            yaw: 0.0,
            zoom: 0.0,
            prev_frame_time: Instant::now(),
            lock_to_in_game: false,
        }
    }

    fn update_camera(
        &mut self,
        ui: &ig::Ui<'_>,
        pipeline: &Pipeline,
        frame: u32,
    ) -> (RotateCamera, bool) {
        let delta_time = self.prev_frame_time.elapsed().as_secs_f32();
        self.prev_frame_time = Instant::now();

        let drag_amount = self.mouse_state.update_drag_amount(ui);
        self.pitch -= drag_amount[1] / 200.0;
        self.yaw -= drag_amount[0] / 200.0;
        let wheel_amount = self.mouse_state.update_wheel_amount(ui);
        self.zoom += wheel_amount / 5.0;
        self.zoom = self.zoom.min(7.0);

        let mario_pos = mario_pos(pipeline, frame);
        let mut target_pos = self.target.unwrap_or(mario_pos);

        let mut fov_y = 45.0f32.to_radians();

        if drag_amount != [0.0, 0.0] || wheel_amount != 0.0 {
            self.lock_to_in_game = false;
        }

        if self.lock_to_in_game {
            target_pos = pipeline
                .timeline()
                .read(frame, "gLakituState.focus")
                .as_f32_3();
            self.target = Some(target_pos);
            let camera_pos = pipeline
                .timeline()
                .read(frame, "gLakituState.pos")
                .as_f32_3();
            let dpos = [
                target_pos[0] - camera_pos[0],
                target_pos[1] - camera_pos[1],
                target_pos[2] - camera_pos[2],
            ];
            let (pitch, yaw) = direction_to_angle(dpos);
            self.pitch = pitch;
            self.yaw = yaw;
            let offset = dpos.iter().map(|c| c.powi(2)).sum::<f32>().sqrt();
            if offset > 0.001 {
                self.zoom = (offset / 1500.0).log(0.5);
            }
            fov_y = pipeline
                .timeline()
                .read(frame, "sFOVState.fov")
                .as_f32()
                .to_radians();
        }

        let offset = 1500.0 * 0.5f32.powf(self.zoom);
        let face_direction = angle_to_direction(self.pitch, self.yaw);

        // TODO: input
        let mut move_dir: [f32; 3] = [0.0, 0.0, 0.0]; // forward, up, right
                                                      //     move_dir[0] += input_float('3d-camera-move-f')
                                                      //     move_dir[0] -= input_float('3d-camera-move-b')
                                                      //     move_dir[1] += input_float('3d-camera-move-u')
                                                      //     move_dir[1] -= input_float('3d-camera-move-d')
                                                      //     move_dir[2] += input_float('3d-camera-move-r')
                                                      //     move_dir[2] -= input_float('3d-camera-move-l')

        if move_dir != [0.0, 0.0, 0.0] || (self.target.is_some() && !self.lock_to_in_game) {
            let mag = move_dir.iter().map(|c| c.powi(2)).sum::<f32>().sqrt();
            if mag > 1.0 {
                move_dir = [move_dir[0] / mag, move_dir[1] / mag, move_dir[2] / mag];
            }

            let max_speed = 50.0 * delta_time * offset.sqrt();
            let f = [self.yaw.sin(), 0.0, self.yaw.cos()];
            let u = [0.0, 1.0, 0.0];
            let r = [-f[2], 0.0, f[0]];

            let mut end_vel = [0.0f32; 3];
            for i in 0..3 {
                end_vel[i] = max_speed * move_dir[0] * f[i]
                    + max_speed * move_dir[1] * u[i]
                    + max_speed * move_dir[2] * r[i];
            }

            let accel = 10.0 * delta_time * offset.sqrt();
            let current_vel = self.target_vel.unwrap_or_default();
            let target_vel = move_toward(current_vel, end_vel, accel);
            self.target_vel = Some(target_vel);
            target_pos = [
                target_pos[0] + target_vel[0],
                target_pos[1] + target_vel[1],
                target_pos[2] + target_vel[2],
            ];
            self.target = Some(target_pos);
            self.lock_to_in_game = false;
        }

        if disableable_button(
            ui,
            im_str!("Lock to Mario"),
            [0.0, 0.0],
            self.target.is_some(),
        ) {
            self.target = None;
            self.target_vel = None;
            self.lock_to_in_game = false;
        }
        ui.same_line(0.0);
        if disableable_button(ui, im_str!("Lakitu"), [0.0, 0.0], !self.lock_to_in_game) {
            self.lock_to_in_game = true;
        }

        let mut camera_pos = [0.0f32; 3];
        for i in 0..3 {
            camera_pos[i] = target_pos[i] - offset * face_direction[i];
        }

        let camera = RotateCamera {
            pos: camera_pos.into(),
            target: target_pos.into(),
            fov_y,
        };
        let show_camera_target = self.target.is_some() && !self.lock_to_in_game;
        (camera, show_camera_target)
    }

    pub(crate) fn render(
        &mut self,
        ui: &ig::Ui<'_>,
        id: &str,
        pipeline: &Pipeline,
        frame: u32,
        framebuffer_size: [f32; 2],
        wall_hitbox_radius: f32,
        hovered_surface: Option<usize>,
        hidden_surfaces: HashSet<usize>,
    ) -> (Scene, Option<usize>) {
        let id_token = ui.push_id(id);

        let (camera, show_camera_target) = self.update_camera(ui, pipeline, frame);
        // TODO: model.rotational_camera_yaw = int(camera.yaw * 0x8000 / math.pi);

        let new_hovered_surface = mouse_ray(ui, &camera)
            .and_then(|mouse_ray| pipeline.trace_ray_to_surface(frame, mouse_ray));

        let scene = render_game(
            pipeline,
            frame,
            viewport(ui, framebuffer_size),
            Camera::Rotate(camera),
            show_camera_target,
            wall_hitbox_radius,
            hovered_surface,
            hidden_surfaces,
        );

        id_token.pop(ui);
        (scene, new_hovered_surface)
    }
}

// TODO: from graphics.py
fn render_game(
    pipeline: &Pipeline,
    frame: u32,
    viewport: Viewport,
    camera: Camera,
    show_camera_target: bool,
    wall_hitbox_radius: f32,
    hovered_surface: Option<usize>,
    hidden_surfaces: HashSet<usize>,
) -> Scene {
    todo!()
}
