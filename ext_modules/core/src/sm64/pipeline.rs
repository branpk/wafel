use super::{
    data_variables::DataVariables,
    layout_extensions::{load_constants, load_object_fields},
    EditRange, RangeEdits, Variable,
};
use crate::{
    dll,
    error::Error,
    memory::{Memory, Value},
    timeline::{Controller, SlotStateMut, Timeline},
};

/// SM64 controller implementation.
#[derive(Debug)]
pub struct SM64Controller {
    data_variables: DataVariables,
    edits: RangeEdits,
}

impl SM64Controller {
    /// Create a new SM64Controller that allows reading/writing the given data variables.
    pub fn new(data_variables: DataVariables) -> Self {
        Self {
            data_variables,
            edits: RangeEdits::new(),
        }
    }
}

impl<M: Memory> Controller<M> for SM64Controller {
    fn apply(&self, state: &mut impl SlotStateMut) -> Result<(), Error> {
        for (variable, value) in self.edits.edits(state.frame()) {
            self.data_variables.set(state, variable, value.clone())?;
        }
        Ok(())
    }
}

/// An abstraction for reading and writing variables.
///
/// Note that writing a value to a variable and then reading the variable does not
/// necessarily result in the original value.
#[derive(Debug)]
pub struct Pipeline<M: Memory> {
    timeline: Timeline<M, SM64Controller>,
}

impl<M: Memory> Pipeline<M> {
    /// Create a new pipeline over the given timeline.
    pub fn new(timeline: Timeline<M, SM64Controller>) -> Self {
        Self { timeline }
    }

    /// Read a variable.
    pub fn read(&self, variable: &Variable) -> Result<Value, Error> {
        let state = self.timeline.frame(variable.try_frame()?)?;
        self.data_variables().get(&state, &variable.without_frame())
    }

    /// Write a variable.
    pub fn write(&mut self, variable: &Variable, value: &Value) -> Result<(), Error> {
        let controller = self.controller_mut(variable)?;
        controller.edits.write(variable, value.clone())?;
        Ok(())
    }

    /// Reset a variable.
    pub fn reset(&mut self, variable: &Variable) -> Result<(), Error> {
        let controller = self.timeline.controller_mut(variable.try_frame()?);
        controller.edits.reset(variable)?;
        Ok(())
    }

    /// Begin a drag operation starting at `source_variable`.
    pub fn begin_drag(
        &mut self,
        source_variable: &Variable,
        source_value: &Value,
    ) -> Result<(), Error> {
        self.controller_mut(source_variable)?
            .edits
            .begin_drag(source_variable, source_value)
    }

    /// Drag from `source_variable` to `target_frame`.
    ///
    /// The ranges will appear to be updated, but won't be committed until `release_drag` is
    /// called.
    pub fn update_drag(&mut self, target_frame: u32) {
        // FIXME: Check frame invalidation for all methods - and avoid unnecessary invalidation
        let controller = self.timeline.controller_mut(target_frame);
        controller.edits.update_drag(target_frame);
    }

    /// End the drag operation, committing range changes.
    pub fn release_drag(&mut self) {
        // FIXME: Invalidation
        let controller = self.timeline.controller_mut(1000000);
        controller.edits.release_drag();
    }

    /// Find the edit range containing a variable, if present.
    pub fn find_edit_range(&self, variable: &Variable) -> Result<Option<&EditRange>, Error> {
        let controller = self.timeline.controller();
        controller.edits.find_variable_range(variable)
    }

    /// Insert a new state at the given frame, shifting edits forward.
    pub fn insert_frame(&mut self, frame: u32) {
        let controller = self.timeline.controller_mut(frame);
        controller.edits.insert_frame(frame);
    }

    /// Delete the state at the given frame, shifting edits backward.
    pub fn delete_frame(&mut self, frame: u32) {
        let controller = self.timeline.controller_mut(frame);
        controller.edits.delete_frame(frame);
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

    fn controller_mut(&mut self, variable: &Variable) -> Result<&mut SM64Controller, Error> {
        let range = self
            .timeline
            .controller()
            .edits
            .find_variable_range(variable)?;
        let range_min = range
            .map(|range| range.frames.start)
            .unwrap_or(variable.try_frame()?);
        Ok(self.timeline.controller_mut(range_min))
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

    load_object_fields(
        memory.data_layout_mut(),
        include_bytes!("../../inline_resources/object_fields.json"),
    )?;
    load_constants(
        memory.data_layout_mut(),
        include_bytes!("../../inline_resources/constants.json"),
    )?;

    let data_variables = DataVariables::all(&memory)?;
    let controller = SM64Controller::new(data_variables);
    let timeline = Timeline::new(memory, base_slot, controller, num_backup_slots)?;
    let pipeline = Pipeline::new(timeline);

    Ok(pipeline)
}
