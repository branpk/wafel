use super::{
    value::{py_object_to_value, value_to_py_object},
    PyAddress, PyEditRange, PyObjectBehavior, PyVariable,
};
use crate::{
    error::Error,
    geo::Point3f,
    geo::Vector3f,
    graphics::scene,
    graphics::scene::Scene,
    sm64::read_objects_to_scene,
    sm64::trace_ray_to_surface,
    sm64::{object_behavior, object_path, read_surfaces_to_scene, ObjectSlot, Pipeline},
};
use lazy_static::lazy_static;
use pyo3::{prelude::*, types::PyBytes};
use std::{collections::HashMap, sync::Mutex};
use wafel_data_type::{Address, IntType, Value};

const NUM_BACKUP_SLOTS: usize = 30;

lazy_static! {
    static ref VALID_PIPELINES: Mutex<Vec<Py<PyPipeline>>> = Mutex::new(Vec::new());
}

/// An abstraction for reading and writing variables.
///
/// Note that writing a value to a variable and then reading the variable does not
/// necessarily result in the original value.
#[pyclass(name = "Pipeline", unsendable)]
#[derive(Debug)]
pub struct PyPipeline {
    valid: Option<ValidPipeline>,
}

#[derive(Debug)]
struct ValidPipeline {
    pipeline: Pipeline,
    symbols_by_address: HashMap<Address, String>,
}

impl PyPipeline {
    fn new(pipeline: Pipeline) -> PyResult<Self> {
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
    /// To help avoid memory leaks, this method also invalidates all existing pipelines that were
    /// created using this method.
    ///
    /// # Safety
    ///
    /// This method is inherently unsafe. See docs for [Timeline::open](wafel_api::Timeline::open).
    #[staticmethod]
    pub unsafe fn load(py: Python<'_>, dll_path: &str) -> PyResult<Py<Self>> {
        let mut valid_pipelines = VALID_PIPELINES.lock().unwrap();

        // Drop all known existing dll::Memory instances for safety
        for pipeline_py in valid_pipelines.drain(..) {
            pipeline_py.borrow_mut(py).invalidate();
        }

        let pipeline = Pipeline::new(dll_path)?;
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
    /// This method is inherently unsafe. See docs for [Timeline::open](wafel_api::Timeline::open).
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
            .into_edits();

        let py_pipeline = Self::load(py, dll_path)?;
        py_pipeline
            .borrow_mut(py)
            .get_mut()
            .pipeline
            .set_edits(edits);

        Ok(py_pipeline)
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
        self.get_mut().pipeline.write(&variable.variable, value)?;
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
        let address = self
            .get()
            .pipeline
            .timeline()
            .try_address(frame, path)
            .map_err(Error::from)?;
        Ok(address.map(|address| PyAddress { address }))
    }

    /// Read from the given path.
    pub fn path_read(&self, py: Python<'_>, frame: u32, path: &str) -> PyResult<PyObject> {
        let value = self
            .get()
            .pipeline
            .timeline()
            .try_read(frame, path)
            .map_err(Error::from)?;
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
            .begin_drag(&source_variable.variable, source_value)?;
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
    pub fn balance_distribution(&mut self, max_run_time_seconds: f32) {
        self.get_mut()
            .pipeline
            .timeline_mut()
            .balance_distribution(max_run_time_seconds);
    }

    /// Return the set of currently loaded frames for debugging purposes.
    pub fn cached_frames(&self) -> Vec<u32> {
        self.get().pipeline.timeline().dbg_cached_frames()
    }

    /// Return the number of frame advances since the timeline was created.
    pub fn num_advances(&self) -> usize {
        0 // TODO
    }

    /// Return the number of slot copies since the timeline was created.
    pub fn num_copies(&self) -> usize {
        0 // TODO
    }

    /// Return the size of the data cache in bytes.
    pub fn data_cache_size(&self) -> usize {
        self.get().pipeline.timeline().dbg_data_cache_size()
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
        let timeline = self.get().pipeline.timeline();
        let data_variables = self.get().pipeline.data_variables();
        let data_type = data_variables.data_type(timeline, &variable.variable)?;
        Ok(data_type.is_int())
    }

    /// Return true if the variable has a float data type.
    pub fn is_float(&self, variable: &PyVariable) -> PyResult<bool> {
        let timeline = self.get().pipeline.timeline();
        let data_variables = self.get().pipeline.data_variables();
        let data_type = data_variables.data_type(timeline, &variable.variable)?;
        Ok(data_type.is_float())
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

    /// Read a null terminated byte string from the given address on the given frame.
    pub fn read_string<'p>(
        &self,
        py: Python<'p>,
        frame: u32,
        address: &PyAddress,
    ) -> PyResult<&'p PyBytes> {
        let timeline = self.get().pipeline.timeline();
        let bytes = timeline
            .try_read_string_at(frame, address.address)
            .map_err(Error::from)?;
        Ok(PyBytes::new(py, &bytes))
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
        let timeline = self.get().pipeline.timeline();
        match object_path(timeline, frame, ObjectSlot(object))? {
            Some(object_path) => {
                let behavior = object_behavior(timeline, frame, &object_path)?;
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
        let timeline = self.get().pipeline.timeline();
        let events = timeline.try_frame_log(frame).map_err(Error::from)?;

        let convert_event = |event: HashMap<String, Value>| -> PyResult<HashMap<String, PyObject>> {
            event
                .into_iter()
                .map(|(key, value)| -> PyResult<_> { Ok((key, value_to_py_object(py, &value)?)) })
                .collect()
        };

        events.into_iter().map(convert_event).collect()
    }

    /// Trace a ray until it hits a surface, and return the surface's index in the surface pool.
    pub fn trace_ray_to_surface(
        &self,
        frame: u32,
        ray: ([f32; 3], [f32; 3]),
    ) -> PyResult<Option<usize>> {
        let timeline = self.get().pipeline.timeline();
        let index = trace_ray_to_surface(
            timeline,
            frame,
            (
                Point3f::from_slice(&ray.0),
                Vector3f::from_row_slice(&ray.1),
            ),
        )?
        .map(|(index, _)| index);
        Ok(index)
    }

    /// Load the SM64 surfaces from the game state and add them to the scene.
    pub fn read_surfaces_to_scene(&self, scene: &mut Scene, frame: u32) -> PyResult<()> {
        let timeline = self.get().pipeline.timeline();
        read_surfaces_to_scene(scene, timeline, frame)?;
        Ok(())
    }

    /// Load the SM64 objects from the game state and add them to the scene.
    pub fn read_objects_to_scene(&self, scene: &mut Scene, frame: u32) -> PyResult<()> {
        let timeline = self.get().pipeline.timeline();
        read_objects_to_scene(scene, timeline, frame)?;
        Ok(())
    }

    /// Add an object path for mario to the scene, using the given frame range.
    pub fn read_mario_path(&self, frame_start: u32, frame_end: u32) -> PyResult<scene::ObjectPath> {
        let timeline = self.get().pipeline.timeline();

        let mut nodes = Vec::new();
        for frame in frame_start..frame_end {
            let pos_coords = timeline
                .try_read(frame, "gMarioState->pos")
                .map_err(Error::from)?
                .try_as_f32_3()
                .map_err(Error::from)?;
            nodes.push(scene::ObjectPathNode {
                pos: Point3f::from_slice(&pos_coords).into(),
                quarter_steps: Vec::new(),
            });
        }

        Ok(scene::ObjectPath {
            nodes,
            root_index: 0,
        })
    }
}
