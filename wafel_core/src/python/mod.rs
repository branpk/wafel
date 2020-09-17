//! SM64-specific Python API for Wafel.
//!
//! The exposed API is **not** safe because of the assumptions made about DLL loading.

use crate::{
    graphics::{
        scene, ImguiCommand, ImguiCommandList, ImguiConfig, ImguiDrawData, IMGUI_FONT_TEXTURE_ID,
    },
    sm64,
};
use bytemuck::{cast_slice, Pod, Zeroable};
pub use imgui_input::*;
pub use pipeline::*;
use pyo3::{prelude::*, wrap_pyfunction};
use sm64::{AdjustedStick, IntendedStick};
use std::{
    collections::{HashMap, HashSet},
    iter,
    num::Wrapping,
    slice,
    time::Instant,
};
pub use variable::*;
use wgpu::util::DeviceExt;
pub use window::*;
use winit::{
    dpi::PhysicalSize,
    event::{
        ElementState::{Pressed, Released},
        Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod error;
mod imgui_input;
mod pipeline;
mod value;
mod variable;
mod window;

// TODO: __str__, __repr__, __eq__, __hash__ for PyObjectBehavior, PyAddress

#[pymodule]
fn core(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPipeline>()?;
    m.add_class::<PyVariable>()?;
    m.add_class::<PyObjectBehavior>()?;
    m.add_class::<PyAddress>()?;
    m.add_class::<scene::Scene>()?;
    m.add_class::<scene::Viewport>()?;
    m.add_class::<scene::RotateCamera>()?;
    m.add_class::<scene::BirdsEyeCamera>()?;
    m.add_class::<scene::ObjectPath>()?;
    m.add_class::<scene::QuarterStep>()?;
    m.add_wrapped(wrap_pyfunction!(open_window_and_run))?;
    m.add_class::<PyAdjustedStick>()?;
    m.add_class::<PyIntendedStick>()?;
    m.add_wrapped(wrap_pyfunction!(stick_raw_to_adjusted))?;
    m.add_wrapped(wrap_pyfunction!(stick_adjusted_to_intended))?;
    m.add_wrapped(wrap_pyfunction!(stick_adjusted_to_raw_euclidean))?;
    m.add_wrapped(wrap_pyfunction!(stick_intended_to_raw_heuristic))?;
    Ok(())
}

#[pyfunction]
pub fn open_window_and_run(title: &str, update_fn: PyObject) -> PyResult<()> {
    open_window_and_run_impl(title, update_fn)
}

#[pyclass(name = AdjustedStick)]
#[derive(Debug, Clone, Copy)]
pub struct PyAdjustedStick {
    #[pyo3(get, set)]
    pub x: f32,
    #[pyo3(get, set)]
    pub y: f32,
    #[pyo3(get, set)]
    pub mag: f32,
}

#[pymethods]
impl PyAdjustedStick {
    #[new]
    pub fn new(x: f32, y: f32, mag: f32) -> Self {
        Self { x, y, mag }
    }
}

#[pyclass(name = IntendedStick)]
#[derive(Debug, Clone, Copy)]
pub struct PyIntendedStick {
    #[pyo3(get, set)]
    pub yaw: i16,
    #[pyo3(get, set)]
    pub mag: f32,
}

#[pymethods]
impl PyIntendedStick {
    #[new]
    pub fn new(yaw: i16, mag: f32) -> Self {
        Self { yaw, mag }
    }
}

#[pyfunction]
pub fn stick_raw_to_adjusted(raw_stick_x: i16, raw_stick_y: i16) -> PyAdjustedStick {
    let adjusted = sm64::stick_raw_to_adjusted(raw_stick_x, raw_stick_y);
    PyAdjustedStick {
        x: adjusted.x,
        y: adjusted.y,
        mag: adjusted.mag,
    }
}

#[pyfunction]
pub fn stick_adjusted_to_intended(
    adjusted: PyAdjustedStick,
    face_yaw: i16,
    camera_yaw: i16,
    squished: bool,
) -> PyIntendedStick {
    let adjusted = AdjustedStick {
        x: adjusted.x,
        y: adjusted.y,
        mag: adjusted.mag,
    };
    let intended = sm64::stick_adjusted_to_intended(
        adjusted,
        Wrapping(face_yaw),
        Wrapping(camera_yaw),
        squished,
    );
    PyIntendedStick {
        yaw: intended.yaw.0,
        mag: intended.mag,
    }
}

#[pyfunction]
pub fn stick_adjusted_to_raw_euclidean(
    target_adjusted_x: f32,
    target_adjusted_y: f32,
) -> (i16, i16) {
    sm64::stick_adjusted_to_raw_euclidean(target_adjusted_x, target_adjusted_y)
}

#[pyfunction]
pub fn stick_intended_to_raw_heuristic(
    intended: PyIntendedStick,
    face_yaw: i16,
    camera_yaw: i16,
    squished: bool,
    relative_to: i16,
) -> (i16, i16) {
    let intended = IntendedStick {
        yaw: Wrapping(intended.yaw),
        mag: intended.mag,
    };
    sm64::stick_intended_to_raw_heuristic(
        intended,
        Wrapping(face_yaw),
        Wrapping(camera_yaw),
        squished,
        Wrapping(relative_to),
    )
}
