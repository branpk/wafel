use wafel_api::Timeline;
use wafel_data_type::Value;

use crate::error::Error;

use super::{data_variables::DataVariables, EditOperation, EditRange, RangeEdits, Variable};

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
    /// Create a new pipeline using the given libsm64 DLL.
    ///
    /// # Safety
    ///
    /// This method is inherently unsafe. See the docs for [wafel_memory::DllGameMemory].
    pub unsafe fn new(dll_path: &str) -> Result<Self, Error> {
        let timeline = Timeline::try_open(dll_path)?;
        let data_variables = DataVariables::all(&timeline)?;
        Ok(Self {
            timeline,
            data_variables,
            range_edits: RangeEdits::new(),
        })
    }

    /// Destroy the pipeline, returning its variable edits.
    pub fn into_edits(self) -> RangeEdits<Variable, Value> {
        self.range_edits
    }

    /// Overwrite all edits with the given edits.
    pub fn set_edits(&mut self, edits: RangeEdits<Variable, Value>) {
        // set range_edits, apply EditOperations for all frames
        todo!()
    }

    /// Read a variable.
    pub fn read(&self, variable: &Variable) -> Result<Value, Error> {
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
                    todo!()
                }
                EditOperation::Insert(frame) => todo!(),
                EditOperation::Delete(frame) => todo!(),
            }
        }
        Ok(())
    }

    /// Write a variable.
    pub fn write(&mut self, variable: &Variable, value: Value) -> Result<(), Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;
        let ops = self.range_edits.write(&column, frame, value);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    /// Reset a variable.
    pub fn reset(&mut self, variable: &Variable) -> Result<(), Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;
        let ops = self.range_edits.reset(&column, frame);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    /// Begin a drag operation starting at `source_variable`.
    pub fn begin_drag(
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

    /// Drag from `source_variable` to `target_frame`.
    ///
    /// The ranges will appear to be updated, but won't be committed until `release_drag` is
    /// called.
    pub fn update_drag(&mut self, target_frame: u32) -> Result<(), Error> {
        let ops = self.range_edits.update_drag(target_frame);
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    /// End the drag operation, committing range changes.
    pub fn release_drag(&mut self) -> Result<(), Error> {
        let ops = self.range_edits.release_drag();
        self.apply_edit_ops(ops)?;
        Ok(())
    }

    /// Find the edit range containing a variable, if present.
    pub fn find_edit_range(&self, variable: &Variable) -> Result<Option<&EditRange<Value>>, Error> {
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
}
