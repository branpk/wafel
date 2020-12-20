use pyo3::prelude::*;
use std::collections::HashMap;
use winit::{
    dpi::PhysicalPosition,
    event::{
        ElementState, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
    },
};

/// Utility for updating imgui keys.
#[derive(Debug)]
pub struct ImguiInput {
    winit_to_glfw_key: HashMap<VirtualKeyCode, u32>,
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
    pub fn handle_event(&mut self, py: Python<'_>, event: &WindowEvent<'_>) -> PyResult<()> {
        let ig = PyModule::import(py, "imgui")?;
        let io = ig.call_method0("get_io")?;

        match event {
            WindowEvent::ReceivedCharacter(c) => self.received_character(io, *c),
            WindowEvent::KeyboardInput { input, .. } => self.keyboard_input(io, *input),
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

    fn keyboard_input(&mut self, io: &PyAny, input: KeyboardInput) -> PyResult<()> {
        let is_down = match input.state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };
        if let Some(winit_key) = input.virtual_keycode {
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
            MouseButton::Other(_) => None,
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

// TODO: Stop using GLFW for input. Will have to define a mapping from VirtualKeyCode
// to int though
const GLFW_WINIT_KEY_MAP: &[(&str, VirtualKeyCode)] = &[
    ("KEY_SPACE", VirtualKeyCode::Space),
    ("KEY_APOSTROPHE", VirtualKeyCode::Apostrophe),
    ("KEY_COMMA", VirtualKeyCode::Comma),
    ("KEY_MINUS", VirtualKeyCode::Minus),
    ("KEY_PERIOD", VirtualKeyCode::Period),
    ("KEY_SLASH", VirtualKeyCode::Slash),
    ("KEY_0", VirtualKeyCode::Key0),
    ("KEY_1", VirtualKeyCode::Key1),
    ("KEY_2", VirtualKeyCode::Key2),
    ("KEY_3", VirtualKeyCode::Key3),
    ("KEY_4", VirtualKeyCode::Key4),
    ("KEY_5", VirtualKeyCode::Key5),
    ("KEY_6", VirtualKeyCode::Key6),
    ("KEY_7", VirtualKeyCode::Key7),
    ("KEY_8", VirtualKeyCode::Key8),
    ("KEY_9", VirtualKeyCode::Key9),
    ("KEY_SEMICOLON", VirtualKeyCode::Semicolon),
    ("KEY_EQUAL", VirtualKeyCode::Equals),
    ("KEY_A", VirtualKeyCode::A),
    ("KEY_B", VirtualKeyCode::B),
    ("KEY_C", VirtualKeyCode::C),
    ("KEY_D", VirtualKeyCode::D),
    ("KEY_E", VirtualKeyCode::E),
    ("KEY_F", VirtualKeyCode::F),
    ("KEY_G", VirtualKeyCode::G),
    ("KEY_H", VirtualKeyCode::H),
    ("KEY_I", VirtualKeyCode::I),
    ("KEY_J", VirtualKeyCode::J),
    ("KEY_K", VirtualKeyCode::K),
    ("KEY_L", VirtualKeyCode::L),
    ("KEY_M", VirtualKeyCode::M),
    ("KEY_N", VirtualKeyCode::N),
    ("KEY_O", VirtualKeyCode::O),
    ("KEY_P", VirtualKeyCode::P),
    ("KEY_Q", VirtualKeyCode::Q),
    ("KEY_R", VirtualKeyCode::R),
    ("KEY_S", VirtualKeyCode::S),
    ("KEY_T", VirtualKeyCode::T),
    ("KEY_U", VirtualKeyCode::U),
    ("KEY_V", VirtualKeyCode::V),
    ("KEY_W", VirtualKeyCode::W),
    ("KEY_X", VirtualKeyCode::X),
    ("KEY_Y", VirtualKeyCode::Y),
    ("KEY_Z", VirtualKeyCode::Z),
    ("KEY_LEFT_BRACKET", VirtualKeyCode::LBracket),
    ("KEY_BACKSLASH", VirtualKeyCode::Backslash),
    ("KEY_RIGHT_BRACKET", VirtualKeyCode::RBracket),
    ("KEY_GRAVE_ACCENT", VirtualKeyCode::Grave),
    // ("KEY_WORLD_1", VirtualKeyCode::WORLD_1),
    // ("KEY_WORLD_2", VirtualKeyCode::WORLD_2),
    ("KEY_ESCAPE", VirtualKeyCode::Escape),
    ("KEY_ENTER", VirtualKeyCode::Return),
    ("KEY_TAB", VirtualKeyCode::Tab),
    ("KEY_BACKSPACE", VirtualKeyCode::Back),
    ("KEY_INSERT", VirtualKeyCode::Insert),
    ("KEY_DELETE", VirtualKeyCode::Delete),
    ("KEY_RIGHT", VirtualKeyCode::Right),
    ("KEY_LEFT", VirtualKeyCode::Left),
    ("KEY_DOWN", VirtualKeyCode::Down),
    ("KEY_UP", VirtualKeyCode::Up),
    ("KEY_PAGE_UP", VirtualKeyCode::PageUp),
    ("KEY_PAGE_DOWN", VirtualKeyCode::PageDown),
    ("KEY_HOME", VirtualKeyCode::Home),
    ("KEY_END", VirtualKeyCode::End),
    ("KEY_CAPS_LOCK", VirtualKeyCode::Capital),
    ("KEY_SCROLL_LOCK", VirtualKeyCode::Scroll),
    ("KEY_NUM_LOCK", VirtualKeyCode::Numlock),
    ("KEY_PRINT_SCREEN", VirtualKeyCode::Snapshot),
    ("KEY_PAUSE", VirtualKeyCode::Pause),
    ("KEY_F1", VirtualKeyCode::F1),
    ("KEY_F2", VirtualKeyCode::F2),
    ("KEY_F3", VirtualKeyCode::F3),
    ("KEY_F4", VirtualKeyCode::F4),
    ("KEY_F5", VirtualKeyCode::F5),
    ("KEY_F6", VirtualKeyCode::F6),
    ("KEY_F7", VirtualKeyCode::F7),
    ("KEY_F8", VirtualKeyCode::F8),
    ("KEY_F9", VirtualKeyCode::F9),
    ("KEY_F10", VirtualKeyCode::F10),
    ("KEY_F11", VirtualKeyCode::F11),
    ("KEY_F12", VirtualKeyCode::F12),
    ("KEY_F13", VirtualKeyCode::F13),
    ("KEY_F14", VirtualKeyCode::F14),
    ("KEY_F15", VirtualKeyCode::F15),
    ("KEY_F16", VirtualKeyCode::F16),
    ("KEY_F17", VirtualKeyCode::F17),
    ("KEY_F18", VirtualKeyCode::F18),
    ("KEY_F19", VirtualKeyCode::F19),
    ("KEY_F20", VirtualKeyCode::F20),
    ("KEY_F21", VirtualKeyCode::F21),
    ("KEY_F22", VirtualKeyCode::F22),
    ("KEY_F23", VirtualKeyCode::F23),
    ("KEY_F24", VirtualKeyCode::F24),
    // ("KEY_F25", VirtualKeyCode::F25),
    ("KEY_KP_0", VirtualKeyCode::Numpad0),
    ("KEY_KP_1", VirtualKeyCode::Numpad1),
    ("KEY_KP_2", VirtualKeyCode::Numpad2),
    ("KEY_KP_3", VirtualKeyCode::Numpad3),
    ("KEY_KP_4", VirtualKeyCode::Numpad4),
    ("KEY_KP_5", VirtualKeyCode::Numpad5),
    ("KEY_KP_6", VirtualKeyCode::Numpad6),
    ("KEY_KP_7", VirtualKeyCode::Numpad7),
    ("KEY_KP_8", VirtualKeyCode::Numpad8),
    ("KEY_KP_9", VirtualKeyCode::Numpad9),
    ("KEY_KP_DECIMAL", VirtualKeyCode::NumpadDecimal),
    ("KEY_KP_DIVIDE", VirtualKeyCode::NumpadDivide),
    ("KEY_KP_MULTIPLY", VirtualKeyCode::NumpadMultiply),
    ("KEY_KP_SUBTRACT", VirtualKeyCode::NumpadSubtract),
    ("KEY_KP_ADD", VirtualKeyCode::NumpadAdd),
    ("KEY_KP_ENTER", VirtualKeyCode::NumpadEnter),
    ("KEY_KP_EQUAL", VirtualKeyCode::NumpadEquals),
    ("KEY_LEFT_SHIFT", VirtualKeyCode::LShift),
    ("KEY_LEFT_CONTROL", VirtualKeyCode::LControl),
    ("KEY_LEFT_ALT", VirtualKeyCode::LAlt),
    ("KEY_LEFT_SUPER", VirtualKeyCode::LWin),
    ("KEY_RIGHT_SHIFT", VirtualKeyCode::RShift),
    ("KEY_RIGHT_CONTROL", VirtualKeyCode::RControl),
    ("KEY_RIGHT_ALT", VirtualKeyCode::RAlt),
    ("KEY_RIGHT_SUPER", VirtualKeyCode::RWin),
    // ("KEY_MENU", VirtualKeyCode::MENU),
];
