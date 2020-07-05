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
use lazy_static::lazy_static;
use std::{collections::HashSet, sync::Mutex};

/// SM64 controller implementation.
#[derive(Debug)]
pub struct SM64Controller<M: Memory> {
    data_variables: DataVariables<M>,
    edits: DirectEdits<M>,
}

impl<M: Memory> SM64Controller<M> {
    /// Create a new SM64Controller that allows reading/writing the given data variables.
    pub fn new(data_variables: DataVariables<M>) -> Self {
        Self {
            data_variables,
            edits: DirectEdits::new(),
        }
    }
}

impl<M: Memory> Controller<M> for SM64Controller<M> {
    fn apply(&self, state: &mut impl SlotStateMut<Memory = M>) -> Result<(), Error> {
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
    timeline: Timeline<M, SM64Controller<M>>,
}

impl<M: Memory> Pipeline<M> {
    /// Create a new pipeline over the given timeline.
    pub fn new(timeline: Timeline<M, SM64Controller<M>>) -> Self {
        Self { timeline }
    }

    /// Read a variable.
    pub fn read(&self, variable: &Variable) -> Result<Value<M::Address>, Error> {
        let state = self.timeline.frame(variable.frame_unwrap())?;
        self.data_variables().get(&state, &variable.without_frame())
    }

    /// Write a variable.
    pub fn write(&mut self, variable: &Variable, value: &Value<M::Address>) {
        let controller = self.timeline.controller_mut(variable.frame_unwrap());
        controller.edits.write(variable, value.clone());
    }

    pub fn data_variables(&self) -> &DataVariables<M> {
        &self.timeline.controller().data_variables
    }
}

lazy_static! {
    static ref DLL_PATHS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
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
    // This is not a perfect check, it is just a sanity check for the likely scenario
    if let Ok(mut dll_paths) = DLL_PATHS.lock() {
        if !dll_paths.insert(dll_path.to_owned()) {
            panic!("Same DLL loaded twice: {}", dll_path);
        }
    }

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
