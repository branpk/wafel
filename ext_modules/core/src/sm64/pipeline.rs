use super::{
    data_variables::DataVariables,
    direct_edits::DirectEdits,
    layout_extensions::{load_constants, load_object_fields},
    Variable,
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
    edits: DirectEdits,
}

impl SM64Controller {
    /// Create a new SM64Controller that allows reading/writing the given data variables.
    pub fn new(data_variables: DataVariables) -> Self {
        Self {
            data_variables,
            edits: DirectEdits::new(),
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
        let controller = self.timeline.controller_mut(variable.try_frame()?);
        controller.edits.write(variable, value.clone())?;
        Ok(())
    }

    /// Reset a variable.
    pub fn reset(&mut self, variable: &Variable) -> Result<(), Error> {
        let controller = self.timeline.controller_mut(variable.try_frame()?);
        controller.edits.reset(variable)?;
        Ok(())
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
