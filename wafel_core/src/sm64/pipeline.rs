use super::{data_variables::DataVariables, EditRange, RangeEdits, Variable};
use crate::{
    dll,
    error::Error,
    memory::Memory,
    timeline::{InvalidatedFrames, SlotStateMut},
};
use wafel_api::Timeline;
use wafel_data_type::Value;

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
        let state = self.timeline.frame(variable.try_frame()?)?;
        self.data_variables().get(&state, &variable.without_frame())
    }

    /// Write a variable.
    pub fn write(&mut self, variable: &Variable, value: &Value) -> Result<(), Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;
        self.timeline.with_controller_mut(|controller| {
            controller.edits.write(&column, frame, value.clone())
        });
        Ok(())
    }

    /// Reset a variable.
    pub fn reset(&mut self, variable: &Variable) -> Result<(), Error> {
        let column = variable.without_frame();
        let frame = variable.try_frame()?;
        self.timeline
            .with_controller_mut(|controller| controller.edits.reset(&column, frame));
        Ok(())
    }

    /// Begin a drag operation starting at `source_variable`.
    pub fn begin_drag(
        &mut self,
        source_variable: &Variable,
        source_value: &Value,
    ) -> Result<(), Error> {
        let column = source_variable.without_frame();
        let source_frame = source_variable.try_frame()?;
        self.timeline.with_controller_mut(|controller| {
            controller
                .edits
                .begin_drag(&column, source_frame, source_value)
        });
        Ok(())
    }

    /// Drag from `source_variable` to `target_frame`.
    ///
    /// The ranges will appear to be updated, but won't be committed until `release_drag` is
    /// called.
    pub fn update_drag(&mut self, target_frame: u32) {
        self.timeline
            .with_controller_mut(|controller| controller.edits.update_drag(target_frame));
    }

    /// End the drag operation, committing range changes.
    pub fn release_drag(&mut self) {
        self.timeline
            .with_controller_mut(|controller| controller.edits.release_drag());
    }

    /// Find the edit range containing a variable, if present.
    pub fn find_edit_range(&self, variable: &Variable) -> Result<Option<&EditRange<Value>>, Error> {
        let controller = self.timeline.controller();
        Ok(controller
            .edits
            .find_range(&variable.without_frame(), variable.try_frame()?))
    }

    /// Insert a new state at the given frame, shifting edits forward.
    pub fn insert_frame(&mut self, frame: u32) {
        self.timeline
            .with_controller_mut(|controller| controller.edits.insert_frame(frame));
    }

    /// Delete the state at the given frame, shifting edits backward.
    pub fn delete_frame(&mut self, frame: u32) {
        self.timeline
            .with_controller_mut(|controller| controller.edits.delete_frame(frame));
    }

    /// Get the data variables for this pipeline.
    pub fn data_variables(&self) -> &DataVariables {
        &self.timeline.controller().data_variables
    }

    /// Get the timeline for this pipeline.
    pub fn timeline(&self) -> &Timeline<M, SM64Controller> {
        &self.timeline
    }

    /// Get the timeline for this pipeline.
    pub fn timeline_mut(&mut self) -> &mut Timeline<M, SM64Controller> {
        &mut self.timeline
    }
}

/// Build a Pipeline using the dll path.
///
/// # Safety
///
/// See `dll::Memory::load`.
pub unsafe fn load_dll_pipeline(
    dll_path: &str,
    num_backup_slots: usize,
) -> Result<Pipeline<dll::Memory>, Error> {
    let (mut memory, base_slot) = dll::Memory::load(dll_path, "sm64_init", "sm64_update")?;
    memory.data_layout_mut().add_sm64_extras()?;

    let data_variables = DataVariables::all(&memory)?;
    let controller = SM64Controller::new(data_variables);
    let timeline = Timeline::new(memory, base_slot, controller, num_backup_slots)?;
    let pipeline = Pipeline::new(timeline);

    Ok(pipeline)
}
