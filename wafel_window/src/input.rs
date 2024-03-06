// TODO: Include physical and logical keys once winit 0.29.0 is released.
// TODO: Custom bindings.

use indexmap::IndexSet;
use wafel_viz::{Rect2, Vec2};
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

/// Access to keyboard state and events.
#[derive(Debug)]
pub struct Input {
    prev_keys_down: IndexSet<KeyCode>,
    keys_down: IndexSet<KeyCode>,
    prev_mouse_buttons_down: IndexSet<MouseButton>,
    mouse_buttons_down: IndexSet<MouseButton>,
    mouse_pos: Option<Vec2>,
    mouse_wheel_delta: Vec2,
}

impl Input {
    pub(crate) fn new() -> Self {
        Self {
            prev_keys_down: IndexSet::new(),
            keys_down: IndexSet::new(),
            mouse_pos: None,
            mouse_wheel_delta: Vec2::zero(),
            prev_mouse_buttons_down: IndexSet::new(),
            mouse_buttons_down: IndexSet::new(),
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        window: &Window,
        event: &WindowEvent,
        egui_consumed: bool,
    ) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !egui_consumed {
                                self.keys_down.insert(key_code);
                            }
                        }
                        ElementState::Released => {
                            self.keys_down.swap_remove(&key_code);
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let logical = position.to_logical::<f32>(window.scale_factor());
                self.mouse_pos = Some([logical.x, logical.y].into());
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match *delta {
                    MouseScrollDelta::LineDelta(dx, dy) => {
                        self.mouse_wheel_delta[0] += dx;
                        self.mouse_wheel_delta[1] += dy;
                    }
                    MouseScrollDelta::PixelDelta(physical) => {
                        let logical = physical.to_logical::<f32>(window.scale_factor());
                        let line_size = 30.0;
                        self.mouse_wheel_delta[0] += logical.x / line_size;
                        self.mouse_wheel_delta[1] += logical.y / line_size;
                    }
                };
            }
            WindowEvent::MouseInput { state, button, .. } => match *state {
                ElementState::Pressed => {
                    self.mouse_buttons_down.insert(*button);
                }
                ElementState::Released => {
                    self.mouse_buttons_down.swap_remove(button);
                }
            },
            _ => {}
        }
    }

    pub(crate) fn end_frame(&mut self) {
        self.prev_keys_down = self.keys_down.clone();
        self.mouse_wheel_delta = Vec2::zero();
        self.prev_mouse_buttons_down = self.mouse_buttons_down.clone();
    }

    /// Returns true if the physical key is currently down.
    pub fn key_down(&self, key_code: KeyCode) -> bool {
        self.keys_down.contains(&key_code)
    }

    /// Returns true if the physical key was pressed this frame.
    pub fn key_pressed(&self, key_code: KeyCode) -> bool {
        !self.prev_keys_down.contains(&key_code) && self.keys_down.contains(&key_code)
    }

    /// Returns true if the physical key was released this frame.
    pub fn key_released(&self, key_code: KeyCode) -> bool {
        self.prev_keys_down.contains(&key_code) && !self.keys_down.contains(&key_code)
    }

    /// Returns true if the mouse button is currently down.
    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    /// Returns true if the mouse button was pressed this frame.
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        !self.prev_mouse_buttons_down.contains(&button) && self.mouse_buttons_down.contains(&button)
    }

    /// Returns true if the mouse button was pressed this frame in the given
    /// logical screen rect.
    pub fn mouse_pressed_in(&self, button: MouseButton, rect: Rect2) -> bool {
        self.mouse_pressed(button) && self.mouse_pos.is_some_and(|pos| rect.contains(pos))
    }

    /// Returns true if the mouse button was released this frame.
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.prev_mouse_buttons_down.contains(&button) && !self.mouse_buttons_down.contains(&button)
    }

    /// Returns the current mouse position in logical coordinates, or `None` if
    /// no cursor move events have been received yet.
    pub fn mouse_pos(&self) -> Option<Vec2> {
        self.mouse_pos
    }

    /// Returns the mouse wheel delta from this frame, in lines/rows.
    pub fn mouse_wheel_delta(&self) -> Vec2 {
        self.mouse_wheel_delta
    }

    /// Returns the mouse wheel delta from this frame if the cursor is in the
    /// given rect, in lines/rows, and otherwise returns `Vec2::zero()`.
    pub fn mouse_wheel_delta_in(&self, rect: Rect2) -> Vec2 {
        if self.mouse_pos().filter(|&p| rect.contains(p)).is_some() {
            self.mouse_wheel_delta
        } else {
            Vec2::zero()
        }
    }
}

/// Helper struct which tracks the state of a mouse drag.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DragState {
    drag_start_pos: Option<Vec2>,
    current_pos: Option<Vec2>,
    prev_pos: Option<Vec2>,
}

impl DragState {
    /// Creates a new `DragState`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the drag state based on the given user input.
    ///
    /// If the user left clicks within `target_rect`, then a drag is started.
    /// The mouse may move outside of the rect during the drag, and ends once
    /// the mouse button is released.
    pub fn update(&mut self, input: &Input, target_rect: Rect2) {
        if input.mouse_pressed_in(MouseButton::Left, target_rect) {
            self.drag_start_pos = input.mouse_pos();
        } else if !input.mouse_down(MouseButton::Left) {
            self.drag_start_pos = None;
        }
        self.prev_pos = self.current_pos;
        self.current_pos = input.mouse_pos();
    }

    /// Returns true if a drag is in progress.
    pub fn is_dragging(&self) -> bool {
        self.drag_start_pos.is_some()
    }

    /// Returns the total amount the mouse has been dragged since the drag
    /// started.
    pub fn drag_amount(&self) -> Vec2 {
        match (self.drag_start_pos, self.current_pos) {
            (Some(drag_start_pos), Some(current_pos)) => current_pos - drag_start_pos,
            _ => Vec2::zero(),
        }
    }

    /// Returns the amount the mouse has been dragged since the previous frame.
    pub fn drag_delta(&self) -> Vec2 {
        match (self.drag_start_pos, self.prev_pos, self.current_pos) {
            (Some(_), Some(prev_pos), Some(current_pos)) => current_pos - prev_pos,
            _ => Vec2::zero(),
        }
    }
}
