use super::{
    value::{py_object_to_value, value_to_py_object},
    PyAddress, PyEditRange, PyObjectBehavior, PyVariable,
};
use crate::{
    dll,
    error::Error,
    graphics::{Scene, Surface, SurfaceType},
    memory::{Address, Memory, Value},
    sm64::{
        frame_log, load_dll_pipeline, object_behavior, object_path, read_surfaces_to_scene,
        ObjectSlot, Pipeline, SM64ErrorCause,
    },
    timeline::{SlotState, State},
};
use lazy_static::lazy_static;
use pyo3::prelude::*;
use std::{collections::HashMap, sync::Mutex};

const NUM_BACKUP_SLOTS: usize = 30;

lazy_static! {
    static ref VALID_PIPELINES: Mutex<Vec<Py<PyPipeline>>> = Mutex::new(Vec::new());
}

/// An abstraction for reading and writing variables.
///
/// Note that writing a value to a variable and then reading the variable does not
/// necessarily result in the original value.
#[pyclass(name = Pipeline, unsendable)]
#[derive(Debug)]
pub struct PyPipeline {
    valid: Option<ValidPipeline>,
}

#[derive(Debug)]
struct ValidPipeline {
    pipeline: Pipeline<dll::Memory>,
    symbols_by_address: HashMap<Address, String>,
}

impl PyPipeline {
    fn new(pipeline: Pipeline<dll::Memory>) -> PyResult<Self> {
        let memory = pipeline.timeline().memory();
        let symbols_by_address = memory
            .all_symbol_address()
            .into_iter()
            .map(|(key, value)| (value, key.to_owned()))
            .collect();

        Ok(Self {
            valid: Some(ValidPipeline {
                pipeline,
                symbols_by_address,
            }),
        })
    }

    fn invalidate(&mut self) -> Option<ValidPipeline> {
        self.valid.take()
    }

    fn get(&self) -> &ValidPipeline {
        self.valid.as_ref().expect("pipeline has been invalidated")
    }

    fn get_mut(&mut self) -> &mut ValidPipeline {
        self.valid.as_mut().expect("pipeline has been invalidated")
    }
}

#[pymethods]
impl PyPipeline {
    /// Load a new pipeline using the given DLL.
    ///
    /// To help ensure DLL safety and avoid memory leaks, this method also invalidates
    /// all existing pipelines that were created using this method.
    ///
    /// # Safety
    ///
    /// See `dll::Memory::load`. As long as the DLL is only loaded via this method,
    /// this method is safe.
    #[staticmethod]
    pub unsafe fn load(py: Python<'_>, dll_path: &str) -> PyResult<Py<Self>> {
        let mut valid_pipelines = VALID_PIPELINES.lock().unwrap();

        // Drop all known existing dll::Memory instances for safety
        for pipeline_py in valid_pipelines.drain(..) {
            pipeline_py.borrow_mut(py).invalidate();
        }

        let pipeline = load_dll_pipeline(dll_path, NUM_BACKUP_SLOTS)?;
        let pipeline_py = Py::new(py, PyPipeline::new(pipeline)?)?;

        valid_pipelines.push(pipeline_py.clone());

        Ok(pipeline_py)
    }

    /// Load a new pipeline using the given DLL, reusing the edits of the given pipeline.
    ///
    /// This method invalidates `prev_pipeline`.
    ///
    /// # Safety
    ///
    /// See `PyPipeline::load`.
    #[staticmethod]
    pub unsafe fn load_reusing_edits(
        py: Python<'_>,
        dll_path: &str,
        prev_pipeline: Py<PyPipeline>,
    ) -> PyResult<Py<Self>> {
        let edits = prev_pipeline
            .borrow_mut(py)
            .invalidate()
            .expect("pipeline has been invalidated")
            .pipeline
            .into_edits()?;

        let py_pipeline = Self::load(py, dll_path)?;
        py_pipeline
            .borrow_mut(py)
            .get_mut()
            .pipeline
            .set_edits(edits);

        Ok(py_pipeline)
    }

    /// Print the data layout to a string for debugging.
    pub fn dump_layout(&self) -> String {
        self.get()
            .pipeline
            .timeline()
            .memory()
            .data_layout()
            .to_string()
    }

    /// Read a variable.
    ///
    /// If the variable is a data variable, the value will be read from memory
    /// on the variable's frame.
    pub fn read(&self, py: Python<'_>, variable: &PyVariable) -> PyResult<PyObject> {
        let value = self.get().pipeline.read(&variable.variable)?;
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
        let value = py_object_to_value(py, &value)?;
        self.get_mut().pipeline.write(&variable.variable, &value)?;
        Ok(())
    }

    /// Reset a variable.
    pub fn reset(&mut self, variable: &PyVariable) -> PyResult<()> {
        self.get_mut().pipeline.reset(&variable.variable)?;
        Ok(())
    }

    /// Get the address for the given path.
    ///
    /// None is only returned if `?` is used in the path.
    pub fn path_address(&self, frame: u32, path: &str) -> PyResult<Option<PyAddress>> {
        let state = self.get().pipeline.timeline().frame(frame)?;
        let address = state.address(path)?.map(|address| PyAddress { address });
        Ok(address)
    }

    /// Read from the given path.
    pub fn path_read(&self, py: Python<'_>, frame: u32, path: &str) -> PyResult<PyObject> {
        let state = self.get().pipeline.timeline().frame(frame)?;
        let value = state.read(path)?;
        let py_object = value_to_py_object(py, &value)?;
        Ok(py_object)
    }

    /// Insert a new state at the given frame, shifting edits forward.
    pub fn insert_frame(&mut self, frame: u32) {
        self.get_mut().pipeline.insert_frame(frame);
    }

    /// Delete the state at the given frame, shifting edits backward.
    pub fn delete_frame(&mut self, frame: u32) {
        self.get_mut().pipeline.delete_frame(frame);
    }

    /// Begin a drag operation starting at `source_variable`.
    pub fn begin_drag(
        &mut self,
        py: Python<'_>,
        source_variable: &PyVariable,
        source_value: PyObject,
    ) -> PyResult<()> {
        let source_value = py_object_to_value(py, &source_value)?;
        self.get_mut()
            .pipeline
            .begin_drag(&source_variable.variable, &source_value)?;
        Ok(())
    }

    /// Drag from `source_variable` to `target_frame`.
    ///
    /// The ranges will appear to be updated, but won't be committed until `release_drag` is
    /// called.
    pub fn update_drag(&mut self, target_frame: u32) {
        self.get_mut().pipeline.update_drag(target_frame);
    }

    /// End the drag operation, committing range changes.
    pub fn release_drag(&mut self) {
        self.get_mut().pipeline.release_drag();
    }

    /// Find the edit range containing a variable, if present.
    pub fn find_edit_range(&self, variable: &PyVariable) -> PyResult<Option<PyEditRange>> {
        let range = self.get().pipeline.find_edit_range(&variable.variable)?;
        Ok(range.cloned().map(|range| PyEditRange { range }))
    }

    /// Set a hotspot, allowing for faster scrolling near the given frame.
    pub fn set_hotspot(&mut self, name: &str, frame: u32) {
        self.get_mut()
            .pipeline
            .timeline_mut()
            .set_hotspot(name, frame);
    }

    /// Perform housekeeping to improve scrolling near hotspots.
    pub fn balance_distribution(&mut self, max_run_time_seconds: f32) -> PyResult<()> {
        self.get_mut()
            .pipeline
            .timeline_mut()
            .balance_distribution(std::time::Duration::from_secs_f32(max_run_time_seconds))?;
        Ok(())
    }

    /// Return the set of currently loaded frames for debugging purposes.
    pub fn cached_frames(&self) -> Vec<u32> {
        self.get().pipeline.timeline().cached_frames()
    }

    /// Return the number of frame advances since the timeline was created.
    pub fn num_advances(&self) -> usize {
        self.get().pipeline.timeline().num_advances()
    }

    /// Return the number of slot copies since the timeline was created.
    pub fn num_copies(&self) -> usize {
        self.get().pipeline.timeline().num_copies()
    }

    /// Return the size of the data cache in bytes.
    pub fn data_cache_size(&self) -> usize {
        self.get().pipeline.timeline().data_size_cache()
    }

    /// Return the label for the variable if it has one.
    pub fn label(&self, variable: &PyVariable) -> PyResult<Option<&str>> {
        let label = self
            .get()
            .pipeline
            .data_variables()
            .label(&variable.variable)?;
        Ok(label)
    }

    /// Return true if the variable has an integer data type.
    pub fn is_int(&self, variable: &PyVariable) -> PyResult<bool> {
        Ok(self
            .get()
            .pipeline
            .data_variables()
            .data_type(&variable.variable)?
            .is_int())
    }

    /// Return true if the variable has a float data type.
    pub fn is_float(&self, variable: &PyVariable) -> PyResult<bool> {
        Ok(self
            .get()
            .pipeline
            .data_variables()
            .data_type(&variable.variable)?
            .is_float())
    }

    /// Return true if the variable is a bit flag.
    pub fn is_bit_flag(&self, variable: &PyVariable) -> PyResult<bool> {
        Ok(self
            .get()
            .pipeline
            .data_variables()
            .flag(&variable.variable)?
            .is_some())
    }

    /// Get the variables in the given group.
    fn variable_group(&self, group: &str) -> Vec<PyVariable> {
        self.get()
            .pipeline
            .data_variables()
            .group(group)
            .map(|variable| PyVariable { variable })
            .collect()
    }

    /// Translate an address into a raw pointer into the base slot.
    ///
    /// # Safety
    ///
    /// This should not be used to write to memory.
    /// This includes any functions that are called through it.
    /// If the given address does not point to static data, no states should be requested from
    /// the timeline while the pointer is alive.
    ///
    /// The pipeline must stay live while this pointer is live.
    pub unsafe fn address_to_base_pointer(
        &self,
        frame: u32,
        address: &PyAddress,
    ) -> PyResult<usize> {
        let timeline = self.get().pipeline.timeline();
        let base_slot = timeline.base_slot(frame)?;
        let pointer: *const u8 = timeline
            .memory()
            .address_to_base_pointer(base_slot.slot(), &address.address)?;
        Ok(pointer as usize)
    }

    /// Return the field offset for a path of the form `struct A.x`.
    pub fn field_offset(&self, path: &str) -> PyResult<usize> {
        let path = self.get().pipeline.timeline().memory().local_path(path)?;
        let offset = path.field_offset()?;
        Ok(offset)
    }

    /// Return the stride of the pointer or array that the path points to.
    pub fn pointer_or_array_stride(&self, path: &str) -> PyResult<Option<usize>> {
        let path = self.get().pipeline.timeline().memory().data_path(path)?;
        let stride = path.concrete_type().stride()?;
        Ok(stride)
    }

    /// Return a map from mario action values to human readable names.
    pub fn action_names(&self) -> HashMap<u32, String> {
        let data_layout = self.get().pipeline.timeline().memory().data_layout();
        data_layout
            .constants
            .iter()
            .filter(|(name, _)| {
                name.starts_with("ACT_")
                    && !name.starts_with("ACT_FLAG_")
                    && !name.starts_with("ACT_GROUP_")
                    && !name.starts_with("ACT_ID_")
            })
            .map(|(name, constant)| {
                (
                    constant.value as u32,
                    name.strip_prefix("ACT_")
                        .unwrap()
                        .replace("_", " ")
                        .to_lowercase(),
                )
            })
            .collect()
    }

    /// Get the object behavior for an object, or None if the object is not active.
    pub fn object_behavior(&self, frame: u32, object: usize) -> PyResult<Option<PyObjectBehavior>> {
        let state = self.get().pipeline.timeline().frame(frame)?;
        match object_path(&state, ObjectSlot(object))? {
            Some(object_path) => {
                let behavior = object_behavior(&state, &object_path)?;
                Ok(Some(PyObjectBehavior { behavior }))
            }
            None => Ok(None),
        }
    }

    /// Get a human readable name for the given object behavior, if possible.
    pub fn object_behavior_name(&self, behavior: &PyObjectBehavior) -> String {
        let address = behavior.behavior.0;
        let symbol = self.get().symbols_by_address.get(&address);

        if let Some(symbol) = symbol {
            symbol.strip_prefix("bhv").unwrap_or(symbol).to_owned()
        } else {
            format!("Object[{}]", address)
        }
    }

    /// Get the wafel frame log for a frame of gameplay.
    ///
    /// The events in the frame log occurred on the previous frame.
    pub fn frame_log(
        &self,
        py: Python<'_>,
        frame: u32,
    ) -> PyResult<Vec<HashMap<String, PyObject>>> {
        let state = self.get().pipeline.timeline().frame(frame)?;
        let events = frame_log(&state)?;

        let convert_event = |event: HashMap<String, Value>| -> PyResult<HashMap<String, PyObject>> {
            event
                .into_iter()
                .map(|(key, value)| -> PyResult<_> { Ok((key, value_to_py_object(py, &value)?)) })
                .collect()
        };

        events.into_iter().map(convert_event).collect()
    }

    pub fn read_surfaces_to_scene(&self, scene: &mut Scene, frame: u32) -> PyResult<()> {
        let state = self.get().pipeline.timeline().frame_uncached(frame)?;
        read_surfaces_to_scene(scene, &state)?;
        Ok(())
    }
}
