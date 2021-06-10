use super::value::value_to_py_object;
use crate::{
    error::Error,
    sm64::{EditRange, ObjectBehavior, ObjectSlot, SM64ErrorCause, SurfaceSlot, Variable},
};
use derive_more::Display;
use pyo3::{basic::CompareOp, prelude::*, types::PyBytes, PyObjectProtocol};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use wafel_api::Value;
use wafel_data_type::Address;

/// An abstract game variable.
#[pyclass(name = "Variable", unsendable)]
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub struct PyVariable {
    pub(crate) variable: Variable,
}

#[pymethods]
impl PyVariable {
    /// Create a variable with the given name with no associated data.
    #[new]
    pub fn new(name: &str) -> Self {
        Self {
            variable: Variable::new(name),
        }
    }

    /// Deserialize a variable from bytes.
    #[staticmethod]
    pub fn from_bytes(src: &[u8]) -> PyResult<PyVariable> {
        let variable: Variable = serde_json::from_slice(src)
            .map_err(|error| Error::from(SM64ErrorCause::VariableSerdeError(error)))?;
        Ok(PyVariable { variable })
    }

    /// Serialize a variable to bytes.
    pub fn to_bytes<'p>(&self, py: Python<'p>) -> PyResult<&'p PyBytes> {
        let bytes = serde_json::to_vec(&self.variable)
            .map_err(|error| Error::from(SM64ErrorCause::VariableSerdeError(error)))?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Get the name of the variable.
    #[getter]
    pub fn name(&self) -> &str {
        self.variable.name.as_ref()
    }

    /// Get the frame for the variable.
    #[getter]
    pub fn frame(&self) -> Option<u32> {
        self.variable.frame
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
            variable: self.variable.with_frame(frame),
        }
    }

    /// Return a copy of the variable but without an associated frame.
    pub fn without_frame(&self) -> Self {
        Self {
            variable: self.variable.without_frame(),
        }
    }

    /// Return a copy of the variable but associated to the given object slot.
    pub fn with_object(&self, object: usize) -> Self {
        Self {
            variable: self.variable.with_object(ObjectSlot(object)),
        }
    }

    /// Return a copy of the variable but without an associated object slot.
    pub fn without_object(&self) -> Self {
        Self {
            variable: self.variable.without_object(),
        }
    }

    /// Return a copy of the variable but associated to the given object behavior.
    pub fn with_object_behavior(&self, behavior: &PyObjectBehavior) -> Self {
        Self {
            variable: self
                .variable
                .with_object_behavior(behavior.behavior.clone()),
        }
    }

    /// Return a copy of the variable but without an associated object behavior.
    pub fn without_object_behavior(&self) -> Self {
        Self {
            variable: self.variable.without_object_behavior(),
        }
    }

    /// Return a copy of the variable but associated to the given surface slot.
    pub fn with_surface(&self, surface: usize) -> Self {
        Self {
            variable: self.variable.with_surface(SurfaceSlot(surface)),
        }
    }

    /// Return a copy of the variable but without an associated surface slot.
    pub fn without_surface(&self) -> Self {
        Self {
            variable: self.variable.without_surface(),
        }
    }
}

#[pyproto]
impl PyObjectProtocol for PyVariable {
    fn __richcmp__(&self, other: PyVariable, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self == &other,
            CompareOp::Ne => self != &other,
            _ => unimplemented!("{:?}", op),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn __str__(&'p self) -> String {
        self.variable.to_string()
    }

    fn __repr__(&'p self) -> String {
        format!("Variable({})", self.variable)
    }
}

/// An opaque representation of an object behavior.
#[pyclass(name = "ObjectBehavior")]
#[derive(Debug)]
pub struct PyObjectBehavior {
    pub(crate) behavior: ObjectBehavior,
}

/// An opaque representation of a memory address.
#[pyclass(name = "Address", unsendable)]
#[derive(Debug, Clone)]
pub struct PyAddress {
    pub(crate) address: Address,
}

/// Information about a variable edit range.
#[pyclass(name = "EditRange")]
#[derive(Debug)]
pub struct PyEditRange {
    pub(crate) range: EditRange<Value>,
}

#[pymethods]
impl PyEditRange {
    /// The id of the range.
    #[getter]
    pub fn id(&self) -> usize {
        self.range.id.0
    }

    /// The start frame of the range (inclusive).
    #[getter]
    pub fn start(&self) -> u32 {
        self.range.frames.start
    }

    /// The end frame of the range (exclusive).
    #[getter]
    pub fn end(&self) -> u32 {
        self.range.frames.end
    }

    /// The value that is applied to the range.
    #[getter]
    pub fn value(&self, py: Python<'_>) -> PyResult<PyObject> {
        value_to_py_object(py, &self.range.value)
    }
}
