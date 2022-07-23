use std::{collections::HashSet, num::Wrapping, time::Instant};

use fast3d::util::{atan2s, coss, sins, Matrixf};
use wafel_data_access::{DataReadable, MemoryLayout};
use wafel_data_type::Angle;
use wafel_memory::MemoryRead;

use crate::{error::VizError, Camera};

#[derive(Debug, Clone, Default)]
pub struct CameraControl {
    mouse_pos: Option<[f32; 2]>,
    in_game_camera: Option<InGameCamera>,
    mario_pos: Option<[f32; 3]>,
    prev_update_movement: Option<Instant>,
    camera_override: Option<CameraOverride>,
    drag_start: Option<DragStart>,
}

#[derive(Debug, Clone)]
struct DragStart {
    mouse_pos: [f32; 2],
    angle: [Angle; 3],
}

#[derive(Debug, Clone)]
struct CameraOverride {
    angle: [Angle; 3],
    dist: f32,
    focus: Focus,
}

#[derive(Debug, Clone, Copy)]
enum Focus {
    InGame,
    Mario,
    Override { pos: [f32; 3], vel: [f32; 3] },
}

#[derive(Debug, Clone, DataReadable)]
#[struct_name("LakituState")]
struct InGameCamera {
    pos: [f32; 3],
    focus: [f32; 3],
    roll: Angle,
}

impl InGameCamera {
    fn dfocus(&self) -> [f32; 3] {
        [
            self.focus[0] - self.pos[0],
            self.focus[1] - self.pos[1],
            self.focus[2] - self.pos[2],
        ]
    }

    fn dist(&self) -> f32 {
        let [dx, dy, dz] = self.dfocus();
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn pitch(&self) -> Angle {
        let [dx, dy, dz] = self.dfocus();
        let xz = (dx * dx + dz * dz).sqrt();
        atan2s(xz, dy)
    }

    fn yaw(&self) -> Angle {
        let [dx, _, dz] = self.dfocus();
        atan2s(dz, dx)
    }

    fn angle(&self) -> [Angle; 3] {
        [self.pitch(), self.yaw(), self.roll]
    }
}

impl Default for Focus {
    fn default() -> Self {
        Self::InGame
    }
}

impl CameraControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_mouse(&mut self, pos: [f32; 2]) {
        self.mouse_pos = Some(pos);
    }

    fn current_angle(&self) -> Option<[Angle; 3]> {
        self.camera_override
            .as_ref()
            .map(|c| c.angle)
            .or_else(|| self.in_game_camera.as_ref().map(|c| c.angle()))
    }

    fn current_dist(&self) -> Option<f32> {
        self.camera_override
            .as_ref()
            .map(|c| c.dist)
            .or_else(|| self.in_game_camera.as_ref().map(|c| c.dist()))
    }

    fn current_focus(&self) -> Option<[f32; 3]> {
        match &self.camera_override {
            Some(camera_override) => match camera_override.focus {
                Focus::InGame => self.in_game_camera.as_ref().map(|c| c.focus),
                Focus::Mario => self.mario_pos,
                Focus::Override { pos, .. } => Some(pos),
            },
            None => self.in_game_camera.as_ref().map(|c| c.focus),
        }
    }

    fn get_or_init_override(&mut self) -> Option<&mut CameraOverride> {
        if let Some(in_game_camera) = &self.in_game_camera {
            Some(self.camera_override.get_or_insert_with(|| CameraOverride {
                angle: in_game_camera.angle(),
                dist: in_game_camera.dist(),
                focus: Focus::InGame,
            }))
        } else {
            None
        }
    }

    pub fn press_mouse_left(&mut self) {
        if self.drag_start.is_none() {
            if let (Some(mouse_pos), Some(angle)) = (self.mouse_pos, self.current_angle()) {
                self.drag_start = Some(DragStart { mouse_pos, angle });
            }
        }
    }

    pub fn release_mouse_left(&mut self) {
        self.drag_start = None;
    }

    pub fn scroll_wheel(&mut self, amount: f32) {
        if let Some(mut dist) = self.current_dist() {
            if dist > 0.001 {
                let mut zoom = (dist / 1500.0).log(0.5);
                zoom += amount / 5.0;
                zoom = zoom.clamp(-5.0, 7.0);
                dist = 0.5f32.powf(zoom) * 1500.0;

                if let Some(camera_override) = self.get_or_init_override() {
                    camera_override.dist = dist;
                }
            }
        }
    }

    pub fn update_movement(&mut self, move_dir: [f32; 3]) {
        let now = Instant::now();
        if let Some(prev) = self.prev_update_movement.replace(now) {
            let dt = now.saturating_duration_since(prev).as_secs_f32();
            if dt <= 0.0 {
                return;
            }

            if move_dir == [0.0; 3]
                && !matches!(
                    self.camera_override.as_ref().map(|c| c.focus),
                    Some(Focus::Override { .. })
                )
            {
                return;
            }

            let [mut dx, mut dy, mut dz] = move_dir;
            let mag = (dx * dx + dy * dy + dz * dz).sqrt();
            if mag > 1.0 {
                dx /= mag;
                dy /= mag;
                dz /= mag;
            }

            if let Some(focus) = self.current_focus() {
                if let Some(camera_override) = self.get_or_init_override() {
                    let max_speed = 50.0 * dt * camera_override.dist.sqrt();
                    dx *= max_speed;
                    dy *= max_speed;
                    dz *= max_speed;

                    let yaw_rotate = Matrixf::rotate_xyz_and_translate(
                        [0.0, 0.0, 0.0],
                        [
                            Wrapping(0),
                            Wrapping(-0x8000) + camera_override.angle[1],
                            Wrapping(0),
                        ],
                    );

                    let end_vel = &yaw_rotate * [dx, dy, dz, 0.0];
                    let end_vel = [end_vel[0], end_vel[1], end_vel[2]];
                    let accel = 10.0 * dt * camera_override.dist.sqrt();

                    let mut cur_vel = match camera_override.focus {
                        Focus::Override { vel, .. } => vel,
                        _ => [0.0; 3],
                    };

                    let dvx = end_vel[0] - cur_vel[0];
                    let dvy = end_vel[1] - cur_vel[1];
                    let dvz = end_vel[2] - cur_vel[2];
                    let dv = (dvx * dvx + dvy * dvy + dvz * dvz).sqrt();

                    if dv <= accel + 0.0001 {
                        cur_vel = end_vel;
                    } else {
                        cur_vel[0] += accel * dvx / dv;
                        cur_vel[1] += accel * dvy / dv;
                        cur_vel[2] += accel * dvz / dv;
                    }

                    camera_override.focus = Focus::Override {
                        pos: [
                            focus[0] + cur_vel[0],
                            focus[1] + cur_vel[1],
                            focus[2] + cur_vel[2],
                        ],
                        vel: cur_vel,
                    };
                }
            }
        }
    }

    pub fn lock_to_in_game_camera(&mut self) {
        self.drag_start = None;
        self.camera_override = None;
    }

    pub fn lock_to_mario(&mut self) {
        if let (Some(angle), Some(dist)) = (self.current_angle(), self.current_dist()) {
            self.camera_override = Some(CameraOverride {
                angle,
                dist,
                focus: Focus::Mario,
            });
        }
    }

    pub fn update(
        &mut self,
        layout: &impl MemoryLayout,
        memory: &impl MemoryRead,
    ) -> Result<Camera, VizError> {
        let in_game_camera_addr = layout.symbol_address("gLakituState")?;
        let in_game_camera: InGameCamera =
            InGameCamera::reader(layout)?.read(memory, in_game_camera_addr)?;
        self.in_game_camera = Some(in_game_camera.clone());

        let mario_pos = layout
            .global_path("gMarioState.pos")?
            .read(memory)?
            .try_as_f32_3()?;
        self.mario_pos = Some(mario_pos);

        if let (Some(drag_state), Some(mouse_pos)) = (&self.drag_start, self.mouse_pos) {
            let drag = [
                mouse_pos[0] - drag_state.mouse_pos[0],
                mouse_pos[1] - drag_state.mouse_pos[1],
            ];
            let drag_dist = (drag[0] * drag[0] + drag[1] * drag[1]).sqrt();
            if self.camera_override.is_some() || drag_dist > 10.0 {
                let [pitch0, yaw0, _] = drag_state.angle;
                let pitch = (pitch0 - Wrapping((drag[1] * 50.0) as i32 as i16))
                    .clamp(Wrapping(-0x3FF0), Wrapping(0x3FF0));
                let yaw = yaw0 - Wrapping((drag[0] * 50.0) as i32 as i16);
                let angle = [pitch, yaw, Wrapping(0)];

                if let Some(camera_override) = self.get_or_init_override() {
                    camera_override.angle = angle;
                }
            }
        }

        if let Some(camera_override) = &self.camera_override {
            let [pitch, yaw, _] = camera_override.angle;
            let focus = match camera_override.focus {
                Focus::InGame => in_game_camera.focus,
                Focus::Mario => mario_pos,
                Focus::Override { pos, .. } => pos,
            };

            let r = camera_override.dist;
            let xz = r * coss(pitch);

            let dx = xz * sins(yaw);
            let dy = r * sins(pitch);
            let dz = xz * coss(yaw);

            let pos = [focus[0] - dx, focus[1] - dy, focus[2] - dz];

            Ok(Camera::LookAt {
                pos,
                focus,
                roll: Wrapping(0),
            })
        } else {
            Ok(Camera::InGame)
        }
    }
}
