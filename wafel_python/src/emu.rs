use std::collections::HashMap;

use pyo3::{prelude::*, PyObjectProtocol};
use wafel_api as api;

use crate::{
    err, py_object_to_value, str_to_version, value_to_py_object, Address, ObjectHitbox, Surface,
};

#[pyclass]
#[derive(Debug)]
pub struct Emu {
    pid: u32,
    inner: api::Emu,
}

#[pymethods]
impl Emu {
    #[staticmethod]
    pub fn attach(pid: u32, base_address: usize, sm64_version: &str) -> PyResult<Self> {
        let sm64_version = str_to_version(sm64_version)?;
        let inner = api::Emu::try_attach(pid, base_address, sm64_version).map_err(err)?;
        Ok(Emu { pid, inner })
    }

    pub fn is_process_open(&self) -> bool {
        self.inner.is_process_open()
    }

    pub fn read(&self, py: Python<'_>, path: &str) -> PyResult<PyObject> {
        let value = self.inner.try_read(path).map_err(err)?;
        let object = value_to_py_object(py, value)?;
        Ok(object)
    }

    pub fn read_string_at(&self, address: Address) -> PyResult<Vec<u8>> {
        self.inner.try_read_string_at(address.inner).map_err(err)
    }

    pub fn address(&self, path: &str) -> PyResult<Option<Address>> {
        let address = self.inner.try_address(path).map_err(err)?;
        Ok(address.map(|inner| Address { inner }))
    }

    pub fn address_to_symbol(&self, address: Address) -> Option<String> {
        self.inner.address_to_symbol(address.inner)
    }

    pub fn data_type(&self, path: &str) -> PyResult<String> {
        let data_type = self.inner.try_data_type(path).map_err(err)?;
        Ok(format!("{}", data_type))
    }

    pub fn write(&mut self, py: Python<'_>, path: &str, value: PyObject) -> PyResult<()> {
        self.inner
            .try_write(path, py_object_to_value(py, value)?)
            .map_err(err)
    }

    pub fn constant(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        let value = self.inner.try_constant(name).map_err(err)?;
        let object = value_to_py_object(py, value)?;
        Ok(object)
    }

    pub fn mario_action_names(&self) -> HashMap<u32, String> {
        self.inner.mario_action_names()
    }

    pub fn surfaces(&self) -> PyResult<Vec<Surface>> {
        let surfaces = self.inner.try_surfaces().map_err(err)?;
        Ok(surfaces
            .into_iter()
            .map(|inner| Surface { inner })
            .collect())
    }

    pub fn object_hitboxes(&self) -> PyResult<Vec<ObjectHitbox>> {
        let hitboxes = self.inner.try_object_hitboxes().map_err(err)?;
        Ok(hitboxes
            .into_iter()
            .map(|inner| ObjectHitbox { inner })
            .collect())
    }
}

#[pyproto]
impl PyObjectProtocol for Emu {
    fn __str__(&'p self) -> String {
        format!("Emu({:?})", self.pid)
    }

    fn __repr__(&'p self) -> String {
        format!("Emu({:?})", self.pid)
    }
}
