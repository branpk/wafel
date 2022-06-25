use wafel_api::{Timeline, Value};

use crate::{
    error::Error,
    geo::{Point3f, Vector3f},
    sm64::{object_behavior, object_path, ObjectBehavior, ObjectSlot},
};

use super::{
    data_variables::DataVariables, trace_ray_to_surface, EditOperation, EditRange, RangeEdits,
    Variable,
};

/// An abstraction for reading and writing variables.
///
/// Note that writing a value to a variable and then reading the variable does not
/// necessarily result in the original value.
#[derive(Debug)]
pub struct Pipeline {
    timeline: Timeline,
    data_variables: DataVariables,
    range_edits: RangeEdits<Variable, Value>,
}

impl Pipeline {
    pub unsafe fn new(dll_path: &str) -> Self {
        Self::try_new(dll_path).unwrap()
    }

    /// Create a new pipeline using the given libsm64 DLL.
    ///
    /// # Safety
    ///
    /// This method is inherently unsafe. See docs for [Timeline::open](wafel_api::Timeline::open).
    pub unsafe fn try_new(dll_path: &str) -> Result<Self, Error> {
        let timeline = Timeline::try_new(dll_path)?;
        let data_variables = DataVariables::all(&timeline)?;
        Ok(Self {
            timeline,
            data_variables,
            range_edits: RangeEdits::new(),
        })
    }

    /// Destroy the pipeline, returning its variable edits.
    pub fn into_edits(self) -> RangeEdits<Variable, Value> {
        self.range_edits.without_drag_state()
    }

    pub fn set_edits(&mut self, edits: RangeEdits<Variable, Value>) {
        self.try_set_edits(edits).unwrap()
    }

    /// Overwrite all edits with the given edits.
    pub fn try_set_edits(&mut self, edits: RangeEdits<Variable, Value>) -> Result<(), Error> {
        self.timeline.reset_all();
        self.range_edits = edits;
        let ops = self.range_edits.initial_ops();
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    pub fn read(&self, variable: &Variable) -> Value {
        self.try_read(variable).unwrap()
    }

    /// Read a variable.
    pub fn try_read(&self, variable: &Variable) -> Result<Value, Error> {
        let frame = variable.try_frame()?;
        let value = self.data_variables.get(&self.timeline, frame, variable)?;
        Ok(value)
    }

    fn apply_edit_ops(&mut self, ops: Vec<EditOperation<Variable, Value>>) -> Result<(), Error> {
        for op in ops {
            match op {
                EditOperation::Write(column, frame, value) => {
                    self.data_variables
                        .set(&mut self.timeline, frame, &column, value)?;
                }
                EditOperation::Reset(column, frame) => {
                    self.data_variables
                        .reset(&mut self.timeline, frame, &column)?;
                }
                EditOperation::Insert(frame) => self.timeline.insert_frame(frame),
                EditOperation::Delete(frame) => self.timeline.delete_frame(frame),
            }
        }
        Ok(())
    }

    pub fn write(&mut self, variable: &Variable, value: Value) {
        self.try_write(variable, value).unwrap()
    }

    /// Write a variable.
    pub fn try_write(&mut self, variable: &Variable, value: Value) -> Result<(), Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;
        let ops = self.range_edits.write(&column, frame, value);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    pub fn reset(&mut self, variable: &Variable) {
        self.try_reset(variable).unwrap();
    }

    /// Reset a variable.
    pub fn try_reset(&mut self, variable: &Variable) -> Result<(), Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;
        let ops = self.range_edits.reset(&column, frame);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    pub fn begin_drag(&mut self, source_variable: &Variable, source_value: Value) {
        self.try_begin_drag(source_variable, source_value).unwrap();
    }

    /// Begin a drag operation starting at `source_variable`.
    pub fn try_begin_drag(
        &mut self,
        source_variable: &Variable,
        source_value: Value,
    ) -> Result<(), Error> {
        let column = source_variable.without_frame();
        let source_frame = source_variable.try_frame()?;
        let ops = self
            .range_edits
            .begin_drag(&column, source_frame, source_value);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    pub fn update_drag(&mut self, target_frame: u32) {
        self.try_update_drag(target_frame).unwrap();
    }

    /// Drag from `source_variable` to `target_frame`.
    ///
    /// The ranges will appear to be updated, but won't be committed until `release_drag` is
    /// called.
    pub fn try_update_drag(&mut self, target_frame: u32) -> Result<(), Error> {
        let ops = self.range_edits.update_drag(target_frame);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    pub fn release_drag(&mut self) {
        self.try_release_drag().unwrap();
    }

    /// End the drag operation, committing range changes.
    pub fn try_release_drag(&mut self) -> Result<(), Error> {
        let ops = self.range_edits.release_drag();
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    pub fn find_edit_range(&self, variable: &Variable) -> Option<&EditRange<Value>> {
        self.try_find_edit_range(variable).unwrap()
    }

    /// Find the edit range containing a variable, if present.
    pub fn try_find_edit_range(
        &self,
        variable: &Variable,
    ) -> Result<Option<&EditRange<Value>>, Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;
        let range = self.range_edits.find_range(&column, frame);
        Ok(range)
    }

    /// Insert a new state at the given frame, shifting edits forward.
    pub fn insert_frame(&mut self, frame: u32) -> Result<(), Error> {
        let ops = self.range_edits.insert_frame(frame);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    /// Delete the state at the given frame, shifting edits backward.
    pub fn delete_frame(&mut self, frame: u32) -> Result<(), Error> {
        let ops = self.range_edits.delete_frame(frame);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    /// Get the data variables for this pipeline.
    pub fn data_variables(&self) -> &DataVariables {
        &self.data_variables
    }

    /// Get the timeline for this pipeline.
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// Get the timeline for this pipeline.
    pub fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.timeline
    }

    pub fn set_hotspot(&mut self, name: &str, frame: u32) {
        self.timeline.set_hotspot(name, frame);
    }

    /// Return true if the variable has an integer data type.
    pub fn is_int(&self, variable: &Variable) -> bool {
        self.data_variables
            .data_type(&self.timeline, variable)
            .unwrap()
            .is_int()
    }

    /// Return true if the variable has a float data type.
    pub fn is_float(&self, variable: &Variable) -> bool {
        self.data_variables
            .data_type(&self.timeline, variable)
            .unwrap()
            .is_float()
    }

    /// Return true if the variable is a bit flag.
    pub fn is_bit_flag(&self, variable: &Variable) -> bool {
        self.data_variables.flag(variable).unwrap().is_some()
    }

    pub fn object_behavior(&self, frame: u32, object: ObjectSlot) -> Option<ObjectBehavior> {
        self.try_object_behavior(frame, object).unwrap()
    }

    /// Get the object behavior for an object, or None if the object is not active.
    pub fn try_object_behavior(
        &self,
        frame: u32,
        object: ObjectSlot,
    ) -> Result<Option<ObjectBehavior>, Error> {
        match object_path(&self.timeline, frame, object)? {
            Some(object_path) => {
                let behavior = object_behavior(&self.timeline, frame, &object_path)?;
                Ok(Some(behavior))
            }
            None => Ok(None),
        }
    }

    /// Get a human readable name for the given object behavior, if possible.
    pub fn object_behavior_name(&self, behavior: &ObjectBehavior) -> String {
        let address = behavior.0;

        match self.timeline.address_to_symbol(address) {
            Some(symbol) => symbol.strip_prefix("bhv").unwrap_or(&symbol).to_owned(),
            None => format!("Object[{}]", address),
        }
    }

    /// Get the variables in the given group.
    pub fn variable_group(&self, group: &str) -> Vec<Variable> {
        self.data_variables.group(group).collect()
    }

    /// Return the label for the variable if it has one.
    pub fn label(&self, variable: &Variable) -> Option<&str> {
        self.data_variables.label(variable).unwrap()
    }

    /// Trace a ray until it hits a surface, and return the surface's index in the surface pool.
    pub fn trace_ray_to_surface(&self, frame: u32, ray: ([f32; 3], [f32; 3])) -> Option<usize> {
        trace_ray_to_surface(
            &self.timeline,
            frame,
            (
                Point3f::from_slice(&ray.0),
                Vector3f::from_row_slice(&ray.1),
            ),
        )
        .unwrap()
        .map(|(index, _)| index)
    }
}
