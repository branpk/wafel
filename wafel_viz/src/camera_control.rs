use std::num::Wrapping;

use fast3d::util::{atan2f, atan2s, coss, sins};
use wafel_data_access::{DataReadable, MemoryLayout};
use wafel_data_type::Angle;
use wafel_memory::MemoryRead;

use crate::{error::VizError, Camera, SM64RenderConfig};

#[derive(Debug, Clone, Default)]
pub struct CameraControl {
    mouse_pos: Option<[f32; 2]>,
    in_game_camera: Option<InGameCamera>,
    angle_override: Option<[Angle; 3]>,
    drag_start: Option<DragStart>,
}

#[derive(Debug, Clone)]
struct DragStart {
    mouse_pos: [f32; 2],
    angle: [Angle; 3],
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

impl CameraControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn resize_view(&mut self, view_size: [u32; 2]) {}

    pub fn move_mouse(&mut self, pos: [f32; 2]) {
        self.mouse_pos = Some(pos);
    }

    fn current_angle(&self) -> Option<[Angle; 3]> {
        self.angle_override
            .or_else(|| self.in_game_camera.as_ref().map(|c| c.angle()))
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

    pub fn update(
        &mut self,
        layout: &impl MemoryLayout,
        memory: &impl MemoryRead,
    ) -> Result<Camera, VizError> {
        if let (Some(drag_state), Some(mouse_pos)) = (&self.drag_start, self.mouse_pos) {
            let drag = [
                mouse_pos[0] - drag_state.mouse_pos[0],
                mouse_pos[1] - drag_state.mouse_pos[1],
            ];
            let drag_dist = (drag[0] * drag[0] + drag[1] * drag[1]).sqrt();
            if self.angle_override.is_some() || drag_dist > 10.0 {
                let [pitch0, yaw0, _] = drag_state.angle;
                let pitch = (pitch0 - Wrapping((drag[1] * 50.0) as i32 as i16))
                    .clamp(Wrapping(-0x3FF0), Wrapping(0x3FF0));
                let yaw = yaw0 - Wrapping((drag[0] * 50.0) as i32 as i16);
                self.angle_override = Some([pitch, yaw, Wrapping(0)]);
            }
        }

        let in_game_camera_addr = layout.symbol_address("gLakituState")?;
        let in_game_camera: InGameCamera =
            InGameCamera::reader(layout)?.read(memory, in_game_camera_addr)?;
        self.in_game_camera = Some(in_game_camera.clone());

        if let Some([pitch, yaw, _]) = self.angle_override {
            let focus = in_game_camera.focus;
            let xyz = in_game_camera.dist();
            let xz = xyz * coss(pitch);

            let dx = xz * sins(yaw);
            let dy = xyz * sins(pitch);
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
