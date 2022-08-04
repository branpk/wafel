#[derive(Debug, Clone)]
pub struct OrthoCameraControl {
    mouse_pos: Option<[f32; 2]>,
    mario_pos: Option<[f32; 3]>,
    focus: Focus,
    span_v: f32,
    drag_start: Option<DragStart>,
}

impl Default for OrthoCameraControl {
    fn default() -> Self {
        Self {
            mouse_pos: None,
            mario_pos: None,
            focus: Focus::default(),
            span_v: 3200.0,
            drag_start: None,
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
        match self.focus {
            Focus::Mario => self.mario_pos,
            Focus::Pos(pos) => Some(pos),
        }
    }

    fn screen_to_world(&self, pos: [f32; 2]) -> Option<[f32; 3]> {}

    pub fn press_mouse_left(&mut self) {
        if self.drag_start.is_none() {
            // if let ()
        }
    }
}
