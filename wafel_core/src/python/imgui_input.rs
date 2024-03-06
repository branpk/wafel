use pyo3::prelude::*;
use std::collections::HashMap;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

/// Utility for updating imgui keys.
#[derive(Debug)]
pub struct ImguiInput {
    winit_to_glfw_key: HashMap<KeyCode, u32>,
    modifier_keys: Vec<(&'static str, u32, u32)>,
}

impl ImguiInput {
    /// Initialize a new `ImguiInput`.
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        let glfw = PyModule::import(py, "glfw")?;

        let mut winit_to_glfw_key = HashMap::new();
        for (glfw_name, winit_key) in GLFW_WINIT_KEY_MAP {
            let glfw_key: u32 = glfw.getattr(glfw_name)?.extract()?;
            winit_to_glfw_key.insert(*winit_key, glfw_key);
        }

        let modifier_key = |imgui_name, glfw_key_name_l, glfw_key_name_r| -> PyResult<_> {
            let glfw_key_l: u32 = glfw.getattr(glfw_key_name_l)?.extract()?;
            let glfw_key_r: u32 = glfw.getattr(glfw_key_name_r)?.extract()?;
            Ok((imgui_name, glfw_key_l, glfw_key_r))
        };
        let modifier_keys = vec![
            modifier_key("key_ctrl", "KEY_LEFT_CONTROL", "KEY_RIGHT_CONTROL")?,
            modifier_key("key_alt", "KEY_LEFT_ALT", "KEY_RIGHT_ALT")?,
            modifier_key("key_shift", "KEY_LEFT_SHIFT", "KEY_RIGHT_SHIFT")?,
            modifier_key("key_super", "KEY_LEFT_SUPER", "KEY_RIGHT_SUPER")?,
        ];

        Ok(Self {
            winit_to_glfw_key,
            modifier_keys,
        })
    }

    /// Initialize the imgui key map.
    pub fn set_key_map(&mut self, py: Python<'_>) -> PyResult<()> {
        let ig = PyModule::import(py, "imgui")?;
        let io = ig.call_method0("get_io")?;

        let glfw = PyModule::import(py, "glfw")?;

        for (imgui_name, glfw_name) in IMGUI_GLFW_KEY_MAP {
            let imgui_key = ig.getattr(imgui_name)?;
            let glfw_key = glfw.getattr(glfw_name)?;
            io.getattr("key_map")?.set_item(imgui_key, glfw_key)?;
        }
        Ok(())
    }

    /// Update delta time for a frame.
    pub fn set_delta_time(&mut self, py: Python<'_>, delta_time: f64) -> PyResult<()> {
        let ig = PyModule::import(py, "imgui")?;
        let io = ig.call_method0("get_io")?;

        io.setattr("delta_time", delta_time)?;
        Ok(())
    }

    /// Set the display size for a frame.
    pub fn set_display_size(&mut self, py: Python<'_>, display_size: (u32, u32)) -> PyResult<()> {
        let ig = PyModule::import(py, "imgui")?;
        let io = ig.call_method0("get_io")?;

        io.setattr("display_size", display_size)?;
        Ok(())
    }

    /// Handle a winit window event.
    pub fn handle_event(&mut self, py: Python<'_>, event: &WindowEvent) -> PyResult<()> {
        let ig = PyModule::import(py, "imgui")?;
        let io = ig.call_method0("get_io")?;

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(text) = &event.text {
                    for c in text.chars() {
                        self.received_character(io, c)?;
                    }
                }
                self.keyboard_input(io, event)
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor_moved(io, *position),
            WindowEvent::MouseWheel { delta, .. } => self.mouse_wheel(io, *delta),
            WindowEvent::MouseInput { state, button, .. } => self.mouse_input(io, *state, *button),
            _ => Ok(()),
        }
    }

    fn received_character(&mut self, io: &PyAny, c: char) -> PyResult<()> {
        let c = c as u32;
        if c < 0x10000 {
            io.call_method1("add_input_character", (c,))?;
        }
        Ok(())
    }

    fn keyboard_input(&mut self, io: &PyAny, event: &KeyEvent) -> PyResult<()> {
        let is_down = match event.state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        if let PhysicalKey::Code(winit_key) = event.physical_key {
            if let Some(&glfw_key) = self.winit_to_glfw_key.get(&winit_key) {
                io.getattr("keys_down")?.set_item(glfw_key, is_down)?;
            }
        }

        for (imgui_prop, glfw_key_l, glfw_key_r) in &self.modifier_keys {
            let down_l: u32 = io.getattr("keys_down")?.get_item(glfw_key_l)?.extract()?;
            let down_r: u32 = io.getattr("keys_down")?.get_item(glfw_key_r)?.extract()?;
            io.setattr(imgui_prop, down_l != 0 || down_r != 0)?;
        }
        Ok(())
    }

    fn cursor_moved(&mut self, io: &PyAny, position: PhysicalPosition<f64>) -> PyResult<()> {
        io.setattr("mouse_pos", (position.x, position.y))?;
        Ok(())
    }

    fn mouse_wheel(&mut self, io: &PyAny, delta: MouseScrollDelta) -> PyResult<()> {
        if let MouseScrollDelta::LineDelta(_, y) = delta {
            let mouse_wheel: f32 = io.getattr("mouse_wheel")?.extract()?;
            io.setattr("mouse_wheel", mouse_wheel + y)?;
        }
        Ok(())
    }

    fn mouse_input(
        &mut self,
        io: &PyAny,
        state: ElementState,
        button: MouseButton,
    ) -> PyResult<()> {
        let is_down = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        let button_index = match button {
            MouseButton::Left => Some(0),
            MouseButton::Right => Some(1),
            MouseButton::Middle => Some(2),
            _ => None,
        };
        if let Some(button_index) = button_index {
            io.getattr("mouse_down")?.set_item(button_index, is_down)?;
        }
        Ok(())
    }
}

const IMGUI_GLFW_KEY_MAP: &[(&str, &str)] = &[
    ("KEY_TAB", "KEY_TAB"),
    ("KEY_LEFT_ARROW", "KEY_LEFT"),
    ("KEY_RIGHT_ARROW", "KEY_RIGHT"),
    ("KEY_UP_ARROW", "KEY_UP"),
    ("KEY_DOWN_ARROW", "KEY_DOWN"),
    ("KEY_PAGE_UP", "KEY_PAGE_UP"),
    ("KEY_PAGE_DOWN", "KEY_PAGE_DOWN"),
    ("KEY_HOME", "KEY_HOME"),
    ("KEY_END", "KEY_END"),
    ("KEY_DELETE", "KEY_DELETE"),
    ("KEY_BACKSPACE", "KEY_BACKSPACE"),
    ("KEY_ENTER", "KEY_ENTER"),
    ("KEY_ESCAPE", "KEY_ESCAPE"),
    ("KEY_A", "KEY_A"),
    ("KEY_C", "KEY_C"),
    ("KEY_V", "KEY_V"),
    ("KEY_X", "KEY_X"),
    ("KEY_Y", "KEY_Y"),
    ("KEY_Z", "KEY_Z"),
];

// TODO: Stop using GLFW for input. Will have to define a mapping from KeyCode
// to int though
const GLFW_WINIT_KEY_MAP: &[(&str, KeyCode)] = &[
    ("KEY_SPACE", KeyCode::Space),
    ("KEY_APOSTROPHE", KeyCode::Quote),
    ("KEY_COMMA", KeyCode::Comma),
    ("KEY_MINUS", KeyCode::Minus),
    ("KEY_PERIOD", KeyCode::Period),
    ("KEY_SLASH", KeyCode::Slash),
    ("KEY_0", KeyCode::Digit0),
    ("KEY_1", KeyCode::Digit1),
    ("KEY_2", KeyCode::Digit2),
    ("KEY_3", KeyCode::Digit3),
    ("KEY_4", KeyCode::Digit4),
    ("KEY_5", KeyCode::Digit5),
    ("KEY_6", KeyCode::Digit6),
    ("KEY_7", KeyCode::Digit7),
    ("KEY_8", KeyCode::Digit8),
    ("KEY_9", KeyCode::Digit9),
    ("KEY_SEMICOLON", KeyCode::Semicolon),
    ("KEY_EQUAL", KeyCode::Equal),
    ("KEY_A", KeyCode::KeyA),
    ("KEY_B", KeyCode::KeyB),
    ("KEY_C", KeyCode::KeyC),
    ("KEY_D", KeyCode::KeyD),
    ("KEY_E", KeyCode::KeyE),
    ("KEY_F", KeyCode::KeyF),
    ("KEY_G", KeyCode::KeyG),
    ("KEY_H", KeyCode::KeyH),
    ("KEY_I", KeyCode::KeyI),
    ("KEY_J", KeyCode::KeyJ),
    ("KEY_K", KeyCode::KeyK),
    ("KEY_L", KeyCode::KeyL),
    ("KEY_M", KeyCode::KeyM),
    ("KEY_N", KeyCode::KeyN),
    ("KEY_O", KeyCode::KeyO),
    ("KEY_P", KeyCode::KeyP),
    ("KEY_Q", KeyCode::KeyQ),
    ("KEY_R", KeyCode::KeyR),
    ("KEY_S", KeyCode::KeyS),
    ("KEY_T", KeyCode::KeyT),
    ("KEY_U", KeyCode::KeyU),
    ("KEY_V", KeyCode::KeyV),
    ("KEY_W", KeyCode::KeyW),
    ("KEY_X", KeyCode::KeyX),
    ("KEY_Y", KeyCode::KeyY),
    ("KEY_Z", KeyCode::KeyZ),
    ("KEY_LEFT_BRACKET", KeyCode::BracketLeft),
    ("KEY_BACKSLASH", KeyCode::Backslash),
    ("KEY_RIGHT_BRACKET", KeyCode::BracketRight),
    ("KEY_GRAVE_ACCENT", KeyCode::Backquote),
    // ("KEY_WORLD_1", KeyCode::WORLD_1),
    // ("KEY_WORLD_2", KeyCode::WORLD_2),
    ("KEY_ESCAPE", KeyCode::Escape),
    ("KEY_ENTER", KeyCode::Enter),
    ("KEY_TAB", KeyCode::Tab),
    ("KEY_BACKSPACE", KeyCode::Backspace),
    ("KEY_INSERT", KeyCode::Insert),
    ("KEY_DELETE", KeyCode::Delete),
    ("KEY_RIGHT", KeyCode::ArrowRight),
    ("KEY_LEFT", KeyCode::ArrowLeft),
    ("KEY_DOWN", KeyCode::ArrowDown),
    ("KEY_UP", KeyCode::ArrowUp),
    ("KEY_PAGE_UP", KeyCode::PageUp),
    ("KEY_PAGE_DOWN", KeyCode::PageDown),
    ("KEY_HOME", KeyCode::Home),
    ("KEY_END", KeyCode::End),
    ("KEY_CAPS_LOCK", KeyCode::CapsLock),
    ("KEY_SCROLL_LOCK", KeyCode::ScrollLock),
    ("KEY_NUM_LOCK", KeyCode::NumLock),
    ("KEY_PRINT_SCREEN", KeyCode::PrintScreen),
    ("KEY_PAUSE", KeyCode::Pause),
    ("KEY_F1", KeyCode::F1),
    ("KEY_F2", KeyCode::F2),
    ("KEY_F3", KeyCode::F3),
    ("KEY_F4", KeyCode::F4),
    ("KEY_F5", KeyCode::F5),
    ("KEY_F6", KeyCode::F6),
    ("KEY_F7", KeyCode::F7),
    ("KEY_F8", KeyCode::F8),
    ("KEY_F9", KeyCode::F9),
    ("KEY_F10", KeyCode::F10),
    ("KEY_F11", KeyCode::F11),
    ("KEY_F12", KeyCode::F12),
    ("KEY_F13", KeyCode::F13),
    ("KEY_F14", KeyCode::F14),
    ("KEY_F15", KeyCode::F15),
    ("KEY_F16", KeyCode::F16),
    ("KEY_F17", KeyCode::F17),
    ("KEY_F18", KeyCode::F18),
    ("KEY_F19", KeyCode::F19),
    ("KEY_F20", KeyCode::F20),
    ("KEY_F21", KeyCode::F21),
    ("KEY_F22", KeyCode::F22),
    ("KEY_F23", KeyCode::F23),
    ("KEY_F24", KeyCode::F24),
    // ("KEY_F25", KeyCode::F25),
    ("KEY_KP_0", KeyCode::Numpad0),
    ("KEY_KP_1", KeyCode::Numpad1),
    ("KEY_KP_2", KeyCode::Numpad2),
    ("KEY_KP_3", KeyCode::Numpad3),
    ("KEY_KP_4", KeyCode::Numpad4),
    ("KEY_KP_5", KeyCode::Numpad5),
    ("KEY_KP_6", KeyCode::Numpad6),
    ("KEY_KP_7", KeyCode::Numpad7),
    ("KEY_KP_8", KeyCode::Numpad8),
    ("KEY_KP_9", KeyCode::Numpad9),
    ("KEY_KP_DECIMAL", KeyCode::NumpadDecimal),
    ("KEY_KP_DIVIDE", KeyCode::NumpadDivide),
    ("KEY_KP_MULTIPLY", KeyCode::NumpadMultiply),
    ("KEY_KP_SUBTRACT", KeyCode::NumpadSubtract),
    ("KEY_KP_ADD", KeyCode::NumpadAdd),
    ("KEY_KP_ENTER", KeyCode::NumpadEnter),
    ("KEY_KP_EQUAL", KeyCode::NumpadEqual),
    ("KEY_LEFT_SHIFT", KeyCode::ShiftLeft),
    ("KEY_LEFT_CONTROL", KeyCode::ControlLeft),
    ("KEY_LEFT_ALT", KeyCode::AltLeft),
    ("KEY_LEFT_SUPER", KeyCode::SuperLeft),
    ("KEY_RIGHT_SHIFT", KeyCode::ShiftRight),
    ("KEY_RIGHT_CONTROL", KeyCode::ControlRight),
    ("KEY_RIGHT_ALT", KeyCode::AltRight),
    ("KEY_RIGHT_SUPER", KeyCode::SuperRight),
    ("KEY_MENU", KeyCode::ContextMenu),
];
