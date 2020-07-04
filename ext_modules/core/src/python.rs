//! SM64-specific Python API for Wafel.
//!
//! The exposed API is **not** safe because of the assumptions made about DLL loading.

use super::sm64::{
    load_dll_pipeline, GenericFrame, GenericVariable, ObjectBehavior, ObjectSlot, Pipeline,
    SM64ErrorCause, SurfaceSlot, Variable,
};
use crate::{dll, error::Error, memory::Value};
use pyo3::{prelude::*, types::PyLong};
use std::convert::TryFrom;

// TODO: __str__, __repr__, __eq__, __hash__ for PyVariable, PyObjectBehavior, PyAddress

#[pymodule]
fn core(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPipeline>()?;
    m.add_class::<PyVariable>()?;
    m.add_class::<PyObjectBehavior>()?;
    m.add_class::<PyAddress>()?;
    Ok(())
}

#[allow(missing_docs)]
mod wafel_error {
    use pyo3::{create_exception, exceptions::Exception};

    create_exception!(wafel, WafelError, Exception);
}
use wafel_error::*;

impl From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        PyErr::new::<WafelError, _>(err.to_string())
    }
}

const NUM_BACKUP_SLOTS: usize = 100;

/// An abstraction for reading and writing variables.
///
/// Note that writing a value to a variable and then reading the variable does not
/// necessarily result in the original value.
#[pyclass(name = Pipeline)]
#[derive(Debug)]
pub struct PyPipeline {
    pipeline: Pipeline<dll::Memory>,
}

#[pymethods]
impl PyPipeline {
    /// Build a Pipeline using the dll path.
    ///
    /// # Safety
    ///
    /// See `dll::Memory::load`.
    #[staticmethod]
    pub unsafe fn load(dll_path: &str) -> PyResult<Self> {
        let pipeline = load_dll_pipeline(dll_path, NUM_BACKUP_SLOTS)?;
        Ok(Self { pipeline })
    }

    /// Read a variable.
    ///
    /// If the variable is a data variable, the value will be read from memory
    /// on the variable's frame.
    pub fn read(&self, py: Python<'_>, variable: &PyVariable) -> PyResult<PyObject> {
        let variable = Variable::try_from(variable.variable.clone())?;
        let value = self.pipeline.read(&variable)?;
        let py_object = value_to_py_object(py, &value)?;
        Ok(py_object)
    }

    /// Write a variable.
    ///
    /// If the variable is a data variable, the value will be truncated and written
    /// to memory on the variable's frame.
    pub fn write(
        &mut self,
        py: Python<'_>,
        variable: &PyVariable,
        value: PyObject,
    ) -> PyResult<()> {
        let variable = Variable::try_from(variable.variable.clone())?;
        let value = py_object_to_value(py, &value)?;
        self.pipeline.write(&variable, &value);
        Ok(())
    }
}

/// An abstract game variable.
#[pyclass(name = Variable)]
#[derive(Debug)]
pub struct PyVariable {
    variable: GenericVariable,
}

#[pymethods]
impl PyVariable {
    /// Create a variable with the given name with no associated data.
    #[new]
    pub fn new(name: &str) -> Self {
        Self {
            variable: Variable::new(name).into(),
        }
    }

    /// Get the name of the variable.
    #[getter]
    pub fn name(&self) -> &str {
        self.variable.name.as_ref()
    }

    /// Get the frame for the variable.
    #[getter]
    pub fn frame(&self) -> Option<u32> {
        match self.variable.frame {
            GenericFrame::Frame(frame) => Some(frame),
            GenericFrame::Absent => None,
        }
    }

    /// Get the object slot for the variable.
    #[getter]
    pub fn object(&self) -> Option<usize> {
        self.variable.object.map(|slot| slot.0)
    }

    /// Get the object behavior for the variable.
    #[getter]
    pub fn object_behavior(&self) -> Option<PyObjectBehavior> {
        self.variable
            .object_behavior
            .clone()
            .map(|behavior| PyObjectBehavior { behavior })
    }

    /// Get the surface slot for the variable.
    #[getter]
    pub fn surface(&self) -> Option<usize> {
        self.variable.surface.map(|slot| slot.0)
    }

    /// Return a copy of the variable but associated with the given frame.
    pub fn with_frame(&self, frame: u32) -> Self {
        Self {
            variable: self.variable.with_frame(frame).into(),
        }
    }

    /// Return a copy of the variable but without an associated frame.
    pub fn without_frame(&self) -> Self {
        Self {
            variable: self.variable.without_frame().into(),
        }
    }

    /// Return a copy of the variable but associated to the given object slot.
    pub fn with_object(&self, object: usize) -> Self {
        Self {
            variable: self.variable.with_object(ObjectSlot(object)).into(),
        }
    }

    /// Return a copy of the variable but without an associated object slot.
    pub fn without_object(&self) -> Self {
        Self {
            variable: self.variable.without_object().into(),
        }
    }

    /// Return a copy of the variable but associated to the given object behavior.
    pub fn with_object_behavior(&self, behavior: &PyObjectBehavior) -> Self {
        Self {
            variable: self
                .variable
                .with_object_behavior(behavior.behavior.clone())
                .into(),
        }
    }

    /// Return a copy of the variable but without an associated object behavior.
    pub fn without_object_behavior(&self) -> Self {
        Self {
            variable: self.variable.without_object_behavior().into(),
        }
    }

    /// Return a copy of the variable but associated to the given surface slot.
    pub fn with_surface(&self, surface: usize) -> Self {
        Self {
            variable: self.variable.with_surface(SurfaceSlot(surface)).into(),
        }
    }

    /// Return a copy of the variable but without an associated surface slot.
    pub fn without_surface(&self) -> Self {
        Self {
            variable: self.variable.without_surface().into(),
        }
    }
}

/// An opaque representation of an object behavior.
#[pyclass(name = ObjectBehavior)]
#[derive(Debug)]
pub struct PyObjectBehavior {
    behavior: ObjectBehavior,
}

/// An opaque representation of a memory address.
#[pyclass(name = Address)]
#[derive(Debug, Clone)]
pub struct PyAddress {
    address: dll::Address,
}

fn value_to_py_object(py: Python<'_>, value: &Value<dll::Address>) -> PyResult<PyObject> {
    match value {
        Value::Int(n) => Ok(n.to_object(py)),
        Value::Float(r) => Ok(r.to_object(py)),
        Value::Address(address) => Ok(PyAddress {
            address: address.clone(),
        }
        .into_py(py)),
        _ => Err(Error::from(SM64ErrorCause::ValueToPython {
            value: value.to_string(),
        })
        .into()),
    }
}

fn py_object_to_value(py: Python<'_>, value: &PyObject) -> PyResult<Value<dll::Address>> {
    if let Ok(long_value) = value.cast_as::<PyLong>(py) {
        Ok(Value::Int(long_value.extract()?))
    } else if let Ok(address) = value.cast_as::<PyAny>(py)?.extract::<PyAddress>() {
        Ok(Value::Address(address.address))
    } else {
        Err(Error::from(SM64ErrorCause::ValueFromPython {
            value: value.cast_as::<PyAny>(py)?.str()?.to_string()?.into(),
        })
        .into())
    }
}
