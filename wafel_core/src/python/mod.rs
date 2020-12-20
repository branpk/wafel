//! SM64-specific Python API for Wafel.
//!
//! The exposed API is **not** safe because of the assumptions made about DLL loading.

use crate::{graphics::scene, sm64};
pub use imgui_input::*;
pub use pipeline::*;
use pyo3::{prelude::*, wrap_pyfunction};
use sm64::{AdjustedStick, IntendedStick};
use std::num::Wrapping;
pub use variable::*;
pub use window::*;

mod error;
mod imgui_input;
mod pipeline;
mod value;
mod variable;
mod window;

// TODO: __str__, __repr__, __eq__, __hash__ for PyObjectBehavior, PyAddress

#[pymodule]
fn wafel_core(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    error::init();
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

/// Open a window, call `update_fn` on each frame, and render the UI and scene(s).
#[pyfunction]
pub fn open_window_and_run(title: &str, update_fn: PyObject) -> PyResult<()> {
    open_window_and_run_impl(title, update_fn)
}

/// The joystick's state after removing the dead zone and capping the magnitude.
#[pyclass(name = AdjustedStick)]
#[derive(Debug, Clone, Copy)]
pub struct PyAdjustedStick {
    /// Adjusted stick x.
    #[pyo3(get, set)]
    pub x: f32,
    /// Adjusted stick y.
    #[pyo3(get, set)]
    pub y: f32,
    /// Adjusted magnitude, [0, 64].
    #[pyo3(get, set)]
    pub mag: f32,
}

#[pymethods]
impl PyAdjustedStick {
    /// Constructor.
    #[new]
    pub fn new(x: f32, y: f32, mag: f32) -> Self {
        Self { x, y, mag }
    }
}

/// The joystick's state as stored in the mario struct.
#[pyclass(name = IntendedStick)]
#[derive(Debug, Clone, Copy)]
pub struct PyIntendedStick {
    /// Intended yaw in world space.
    #[pyo3(get, set)]
    pub yaw: i16,
    /// Intended magnitude, normally [0, 32].
    #[pyo3(get, set)]
    pub mag: f32,
}

#[pymethods]
impl PyIntendedStick {
    /// Constructor.
    #[new]
    pub fn new(yaw: i16, mag: f32) -> Self {
        Self { yaw, mag }
    }
}

/// In-game calculation converting raw stick inputs to adjusted.
#[pyfunction]
pub fn stick_raw_to_adjusted(raw_stick_x: i16, raw_stick_y: i16) -> PyAdjustedStick {
    let adjusted = sm64::stick_raw_to_adjusted(raw_stick_x, raw_stick_y);
    PyAdjustedStick {
        x: adjusted.x,
        y: adjusted.y,
        mag: adjusted.mag,
    }
}

/// In-game calculation converting adjusted stick to intended.
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

/// Return the raw stick value whose adjusted stick is closest to the given
/// adjusted inputs, based on Euclidean distance.
#[pyfunction]
pub fn stick_adjusted_to_raw_euclidean(
    target_adjusted_x: f32,
    target_adjusted_y: f32,
) -> (i16, i16) {
    sm64::stick_adjusted_to_raw_euclidean(target_adjusted_x, target_adjusted_y)
}

/// Find a raw josytick value that approximately maps to the given intended inputs.
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
