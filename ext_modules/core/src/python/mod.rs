//! SM64-specific Python API for Wafel.
//!
//! The exposed API is **not** safe because of the assumptions made about DLL loading.

pub use pipeline::*;
use pyo3::prelude::*;
pub use variable::*;

mod error;
mod pipeline;
mod value;
mod variable;

// TODO: __str__, __repr__, __eq__, __hash__ for PyObjectBehavior, PyAddress

#[pymodule]
fn core(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPipeline>()?;
    m.add_class::<PyVariable>()?;
    m.add_class::<PyObjectBehavior>()?;
    m.add_class::<PyAddress>()?;
    Ok(())
}
