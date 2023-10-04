#[derive(Debug, Clone, Default)]
pub struct OrthoCameraControl {
    mouse_pos: Option<[f32; 2]>,
    mario_pos: Option<[f32; 3]>,
    state: CameraState,
    drag_start: Option<DragStart>,
}

#[derive(Debug, Clone)]
struct CameraState {
    focus: Focus,
    forward_pos: Option<f32>,
    forward: [f32; 3],
    upward: [f32; 3],
    span_v: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            focus: Focus::default(),
            forward_pos: None,
            forward: [0.0, -1.0, 0.0],
            upward: [1.0, 0.0, 0.0],
            span_v: 3200.0,
        }
    }
}

#[derive(Debug, Clone)]
struct DragStart {
    mouse_world_pos: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
enum Focus {
    Mario,
    Pos([f32; 3]),
}

impl Default for Focus {
    fn default() -> Self {
        Self::Mario
    }
}

impl OrthoCameraControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_mouse(&mut self, pos: [f32; 2]) {
        self.mouse_pos = Some(pos);
    }

    fn current_focus(&self) -> Option<[f32; 3]> {
        match self.state.focus {
            Focus::Mario => self.mario_pos,
            Focus::Pos(pos) => Some(pos),
        }
    }

    fn screen_to_world(&self, pos: [f32; 2]) -> Option<[f32; 3]> {
        todo!()
    }

    pub fn press_mouse_left(&mut self) {
        if self.drag_start.is_none() {
            // if let ()
        }
    }
}
