use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use pyo3::{basic::CompareOp, prelude::*, PyObjectProtocol};
use wafel_api as api;

use crate::{err, str_to_version, WafelError};

#[pyclass]
#[derive(Debug)]
pub struct M64Metadata {
    inner: api::M64Metadata,
}

#[pymethods]
impl M64Metadata {
    #[new]
    pub fn new(crc_code: u32, country_code: u8) -> Self {
        Self {
            inner: api::M64Metadata::new(crc_code, country_code),
        }
    }

    #[staticmethod]
    pub fn with_version(version: &str) -> PyResult<Self> {
        Ok(Self {
            inner: api::M64Metadata::with_version(str_to_version(version)?),
        })
    }

    pub fn crc_code(&self) -> u32 {
        self.inner.crc_code()
    }

    pub fn set_crc_code(&mut self, crc_code: u32) {
        self.inner.set_crc_code(crc_code);
    }

    pub fn country_code(&self) -> u8 {
        self.inner.country_code()
    }

    pub fn set_country_code(&mut self, country_code: u8) {
        self.inner.set_country_code(country_code);
    }

    pub fn version(&self) -> Option<String> {
        self.inner.version().map(version_to_str)
    }

    pub fn set_version(&mut self, version: &str) -> PyResult<()> {
        self.inner.set_version(str_to_version(version)?);
        Ok(())
    }

    pub fn author(&self) -> &str {
        self.inner.author()
    }

    pub fn set_author(&mut self, author: &str) -> PyResult<()> {
        self.inner.try_set_author(author).map_err(err)?;
        Ok(())
    }

    pub fn description(&self) -> &str {
        self.inner.description()
    }

    pub fn set_description(&mut self, description: &str) -> PyResult<()> {
        self.inner.try_set_description(description).map_err(err)?;
        Ok(())
    }

    pub fn rerecords(&self) -> u32 {
        self.inner.rerecords()
    }

    pub fn set_rerecords(&mut self, rerecords: u32) {
        self.inner.set_rerecords(rerecords);
    }

    pub fn add_rerecords(&mut self, rerecords: u32) {
        self.inner.add_rerecords(rerecords);
    }
}

#[pyproto]
impl PyObjectProtocol for M64Metadata {
    fn __str__(&'p self) -> String {
        format!("{}", self.inner)
    }

    fn __repr__(&'p self) -> String {
        format!("{}", self.inner)
    }
}

fn version_to_str(version: api::SM64Version) -> String {
    match version {
        api::SM64Version::JP => "jp".to_string(),
        api::SM64Version::US => "us".to_string(),
        api::SM64Version::EU => "eu".to_string(),
        api::SM64Version::SH => "sh".to_string(),
    }
}

#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Input {
    inner: api::Input,
}

#[pymethods]
impl Input {
    #[new]
    pub fn new(buttons: u16, stick_x: u8, stick_y: u8) -> Self {
        Self {
            inner: api::Input {
                buttons,
                stick_x,
                stick_y,
            },
        }
    }

    pub fn copy(&self) -> Self {
        Self { inner: self.inner }
    }

    #[getter]
    pub fn buttons(&self) -> u16 {
        self.inner.buttons
    }

    #[setter]
    pub fn set_buttons(&mut self, buttons: u16) {
        self.inner.buttons = buttons;
    }

    #[getter]
    pub fn stick_x(&self) -> u8 {
        self.inner.stick_x
    }

    #[setter]
    pub fn set_stick_x(&mut self, stick_x: u8) {
        self.inner.stick_x = stick_x;
    }

    #[getter]
    pub fn stick_y(&self) -> u8 {
        self.inner.stick_y
    }

    #[setter]
    pub fn set_stick_y(&mut self, stick_y: u8) {
        self.inner.stick_y = stick_y;
    }
}

#[pyproto]
impl PyObjectProtocol for Input {
    fn __str__(&'p self) -> String {
        format!("{}", self.inner)
    }

    fn __repr__(&'p self) -> String {
        format!("{}", self.inner)
    }

    fn __hash__(&'p self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn __richcmp__(&'p self, other: Input, op: CompareOp) -> PyResult<bool> {
        match op {
            CompareOp::Eq => Ok(self == &other),
            CompareOp::Ne => Ok(self != &other),
            _ => Err(PyErr::new::<WafelError, _>("unimplemented comparison")),
        }
    }
}

pub fn load_m64(filename: &str) -> PyResult<(M64Metadata, Vec<Input>)> {
    let (metadata, inputs) = api::try_load_m64(filename).map_err(err)?;
    Ok((
        M64Metadata { inner: metadata },
        inputs.into_iter().map(|inner| Input { inner }).collect(),
    ))
}

pub fn save_m64(filename: &str, metadata: &M64Metadata, inputs: Vec<Input>) -> PyResult<()> {
    api::try_save_m64(
        filename,
        &metadata.inner,
        &inputs
            .into_iter()
            .map(|input| input.inner)
            .collect::<Vec<_>>(),
    )
    .map_err(err)
}
