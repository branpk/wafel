use super::PyAddress;
use crate::{error::Error, memory::Value, sm64::SM64ErrorCause};
use pyo3::{
    prelude::*,
    types::{IntoPyDict, PyFloat, PyList, PyLong},
};

pub fn value_to_py_object(py: Python<'_>, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Int(n) => Ok(n.to_object(py)),
        Value::Float(r) => Ok(r.to_object(py)),
        Value::String(s) => Ok(s.to_object(py)),
        Value::Address(address) => Ok(PyAddress {
            address: (*address).into(),
        }
        .into_py(py)),
        Value::Struct { fields } => Ok(fields
            .iter()
            .map(|(name, value)| value_to_py_object(py, value).map(|object| (name, object)))
            .collect::<PyResult<Vec<_>>>()?
            .into_py_dict(py)
            .to_object(py)),
        Value::Array(items) => {
            let objects: Vec<PyObject> = items
                .iter()
                .map(|value| value_to_py_object(py, value))
                .collect::<PyResult<_>>()?;
            Ok(PyList::new(py, objects).to_object(py))
        }
    }
}

pub fn py_object_to_value(py: Python<'_>, value: &PyObject) -> PyResult<Value> {
    if value.is_none(py) {
        Ok(Value::Null)
    } else if let Ok(long_value) = value.cast_as::<PyLong>(py) {
        Ok(Value::Int(long_value.extract()?))
    } else if let Ok(float_value) = value.cast_as::<PyFloat>(py) {
        Ok(Value::Float(float_value.extract()?))
    } else if let Ok(address) = value.cast_as::<PyAny>(py)?.extract::<PyAddress>() {
        Ok(Value::Address(address.address.into()))
    } else {
        Err(Error::from(SM64ErrorCause::ValueFromPython {
            value: value.cast_as::<PyAny>(py)?.str()?.to_string()?.into(),
        })
        .into())
    }
}
