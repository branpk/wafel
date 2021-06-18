use std::collections::HashMap;

use pyo3::{prelude::*, PyObjectProtocol};
use wafel_api as api;

use crate::{
    convert_frame_log, err, py_object_to_value, value_to_py_object, Address, ObjectHitbox, Surface,
};

#[pyclass]
#[derive(Debug)]
pub struct Game {
    dll_path: String,
    inner: api::Game,
}

#[pymethods]
impl Game {
    #[new]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn open(dll_path: &str) -> PyResult<Self> {
        let inner = api::Game::try_new(dll_path).map_err(err)?;
        Ok(Game {
            dll_path: dll_path.to_string(),
            inner,
        })
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

    pub fn frame(&self) -> u32 {
        self.inner.frame()
    }

    pub fn advance(&mut self) {
        self.inner.advance()
    }

    pub fn advance_n(&mut self, num_frames: u32) {
        self.inner.advance_n(num_frames)
    }

    pub fn save_state(&self) -> SaveState {
        let inner = self.inner.save_state();
        SaveState { inner }
    }

    pub fn load_state(&mut self, state: &SaveState) -> PyResult<()> {
        self.inner.try_load_state(&state.inner).map_err(err)
    }

    pub fn rerecords(&self) -> u32 {
        self.inner.rerecords()
    }

    pub fn constant(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        let value = self.inner.try_constant(name).map_err(err)?;
        let object = value_to_py_object(py, value)?;
        Ok(object)
    }

    pub fn mario_action_names(&self) -> HashMap<u32, String> {
        self.inner.mario_action_names()
    }

    pub fn frame_log(&self, py: Python<'_>) -> PyResult<Vec<HashMap<String, PyObject>>> {
        let frame_log = self.inner.try_frame_log().map_err(err)?;
        let object = convert_frame_log(py, frame_log)?;
        Ok(object)
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
impl PyObjectProtocol for Game {
    fn __str__(&'p self) -> String {
        format!("Game({:?})", self.dll_path)
    }

    fn __repr__(&'p self) -> String {
        format!("Game({:?})", self.dll_path)
    }
}

#[pyclass]
#[derive(Debug)]
pub struct SaveState {
    inner: api::SaveState,
}

#[pymethods]
impl SaveState {
    fn frame(&self) -> u32 {
        self.inner.frame()
    }
}
